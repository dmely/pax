use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    forward_to_deserialize_any,
};

use pax_lang::{Parser, PaxParser, Rule};
use pax_runtime_api::constants::{COLOR_CHANNEL, INTEGER, PERCENT};

use crate::constants::NUMERIC;

use super::{
    error::{Error, Result},
    Deserializer,
};

#[derive(Debug)]
pub struct PaxColor {
    pub color_func: String,
    pub args: Vec<ColorFuncArg>,
}

#[derive(Debug)]
pub enum ColorFuncArg {
    Percent(String),
    Integer(String),
    Rotation(String),
}

impl<'de> EnumAccess<'de> for crate::deserializer::helpers::PaxColor {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(crate::deserializer::helpers::PrimitiveDeserializer::new(
            self.color_func.as_str(),
        ))?;
        Ok((val, self))
    }
}

impl<'de> VariantAccess<'de> for crate::deserializer::helpers::PaxColor {
    type Error = Error;

    // Handle color constants like Color::RED
    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    // Handle seq args like Color::rgb(255,0,0) or Color::rgb(100%, 0%, 0%)
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let seq = self
            .args
            .iter()
            .map(|cc| match cc {
                ColorFuncArg::Rotation(val) => PaxSeqArg::String(val.to_string()),
                ColorFuncArg::Percent(val) => PaxSeqArg::Enum(PaxEnum {
                    identifier: Some(COLOR_CHANNEL.to_string()),
                    variant: PERCENT.to_string(),
                    args: Some(val.to_string()),
                }),
                ColorFuncArg::Integer(val) => PaxSeqArg::Enum(PaxEnum {
                    identifier: Some(COLOR_CHANNEL.to_string()),
                    variant: INTEGER.to_string(),
                    args: Some(val.to_string()),
                }),
            })
            .collect();

        visitor.visit_seq(PaxSeq::new(seq))
    }

    // Color::rgb(only_one_arg)
    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        unreachable!(); //Incorrect color syntax
    }

    // Color::rgb { r: ... } (not supported)
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unreachable!(); //Incorrect color syntax
    }
}

#[derive(Clone)]
pub struct PaxEnum {
    //a None-identifier allows us to manage tuple-structs as enums, e.g. `Percent(10)`
    identifier: Option<String>,
    variant: String,
    args: Option<String>,
}

impl<'de> de::Deserializer<'de> for PaxEnum {
    type Error = Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self.clone())
    }
}

impl PaxEnum {
    pub fn new(identifier: Option<String>, variant: String, args: Option<String>) -> Self {
        PaxEnum {
            identifier,
            variant,
            args,
        }
    }

    pub fn from_string(input: String) -> Self {
        let mut pairs = PaxParser::parse(Rule::literal_enum_value, &input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner()
            .rev();
        let end = pairs.next().unwrap();
        let mut args: Option<String> = None;
        let second = pairs.next().unwrap().as_str().to_string();
        let variant;
        let identifier;
        match end.as_rule() {
            Rule::literal_enum_args_list => {
                args = Some(end.as_str().to_owned());
                variant = second;
                identifier = pairs.next().unwrap().as_str().to_string();
            }
            Rule::identifier => {
                variant = end.as_str().to_owned();
                identifier = second;
            }
            _ => {
                unreachable!(
                    "Unexpected rule: {:?}, original value: {:?}",
                    end.as_rule(),
                    end.as_str()
                )
            }
        }
        PaxEnum {
            identifier: Some(identifier),
            variant,
            args,
        }
    }
}

impl<'de> EnumAccess<'de> for PaxEnum {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(PrimitiveDeserializer::new(self.variant.as_str()))?;
        Ok((val, self))
    }
}

impl<'de> VariantAccess<'de> for PaxEnum {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        if let Some(i) = self.identifier {
            if i == NUMERIC {
                return seed.deserialize(PrimitiveDeserializer::new(&self.args.unwrap()));
            }
        }

        seed.deserialize(Deserializer::from_string(self.args.unwrap()))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Ok(mut ast) =
            PaxParser::parse(Rule::literal_enum_args_list, &self.args.clone().unwrap())
        {
            let elements = ast
                .next()
                .unwrap()
                .into_inner()
                .map(|x| PaxSeqArg::String(x.as_str().to_owned()))
                .collect::<Vec<PaxSeqArg>>();
            visitor.visit_seq(PaxSeq::new(elements))
        } else {
            panic!("Failed to parse: {}", &self.args.unwrap())
        }
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut map = PaxObject::from_string(self.args.unwrap());
        visitor.visit_map(&mut map)
    }
}

struct PrimitiveDeserializer {
    input: String,
}

impl PrimitiveDeserializer {
    fn new(input: &str) -> Self {
        PrimitiveDeserializer {
            input: input.to_owned(),
        }
    }
}

impl<'de> de::Deserializer<'de> for PrimitiveDeserializer {
    type Error = Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Ok(mut ast) = PaxParser::parse(Rule::literal_number_float, &self.input) {
            visitor.visit_f64(ast.next().unwrap().as_str().trim().parse::<f64>().unwrap())
        } else if let Ok(mut ast) = PaxParser::parse(Rule::literal_number_integer, &self.input) {
            visitor.visit_i64(ast.next().unwrap().as_str().trim().parse::<i64>().unwrap())
        } else if let Ok(mut ast) = PaxParser::parse(Rule::literal_boolean, &self.input) {
            visitor.visit_bool(ast.next().unwrap().as_str().trim().parse::<bool>().unwrap())
        } else if let Ok(mut ast) = PaxParser::parse(Rule::inner, &self.input) {
            visitor.visit_str(ast.next().unwrap().as_str().trim())
        } else {
            panic!("Failed to parse: {}", &self.input)
        }
    }

    fn deserialize_identifier<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(&self.input)
    }
}

pub struct PaxSeq {
    elements: Vec<PaxSeqArg>,
    index: usize,
}

#[derive(Clone)]
pub enum PaxSeqArg {
    String(String),
    Enum(PaxEnum),
}

impl PaxSeq {
    pub fn new(elements: Vec<PaxSeqArg>) -> Self {
        PaxSeq { elements, index: 0 }
    }
}

impl<'de> SeqAccess<'de> for PaxSeq {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.index < self.elements.len() {
            let elem = &self.elements[self.index];

            let val = match elem {
                PaxSeqArg::Enum(pax_enum) => seed.deserialize(pax_enum.clone())?,
                PaxSeqArg::String(str) => {
                    seed.deserialize(Deserializer::from_string(str.to_string()))?
                }
            };

            self.index += 1;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }
}

pub struct PaxObject {
    name: Option<String>,
    elements: Vec<(String, String)>,
    index: usize,
}

impl PaxObject {
    pub fn from_string(input: String) -> Self {
        let mut pairs = PaxParser::parse(Rule::literal_object, &input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        // remove optional identifier
        let name = if let Rule::pascal_identifier = pairs.peek().unwrap().as_rule() {
            Some(pairs.next().unwrap().as_str().to_string())
        } else {
            None
        };

        let mut elements = Vec::new();
        for pair in pairs {
            if let Rule::settings_key_value_pair = pair.as_rule() {
                let mut inner = pair.into_inner();
                let key = inner.next().unwrap().into_inner().next().unwrap();
                let val = inner.next().unwrap().into_inner().next().unwrap();
                elements.push((key.as_str().to_string(), val.as_str().to_string()));
            }
        }

        PaxObject {
            name,
            elements,
            index: 0,
        }
    }

    pub fn new(name: Option<String>, elements: Vec<(String, String)>) -> Self {
        PaxObject {
            name,
            elements,
            index: 0,
        }
    }
}

impl<'de> MapAccess<'de> for PaxObject {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.index < self.elements.len() {
            let val = seed.deserialize(Deserializer::from_string(
                self.elements[self.index].0.clone(),
            ))?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        if self.index < self.elements.len() {
            let val = seed.deserialize(Deserializer::from_string(
                self.elements[self.index].1.clone(),
            ))?;
            self.index += 1;
            Ok(val)
        } else {
            unreachable!("next_key_seed must be called before next_value_seed");
        }
    }
}

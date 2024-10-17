// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use serde_json::value::{Map, Value};

const KEY_SEPARATOR: &str = "_";

#[inline]
pub fn flatten(to_flatten: Value) -> Result<Value, anyhow::Error> {
    flatten_with_level(to_flatten, 0)
}

/// Flattens the provided JSON object (`current`).
///
/// It will return an error if flattening the object would make two keys to be
/// the same, overwriting a value. It will alre return an error if the JSON
/// value passed it's not an object.
///
/// # Errors
/// Will return `Err` if `to_flatten` it's not an object, or if flattening the
/// object would result in two or more keys colliding.
pub fn flatten_with_level(to_flatten: Value, max_level: u32) -> Result<Value, anyhow::Error> {
    // quick check to see if we have an object`
    let to_flatten = match to_flatten {
        Value::Object(v) => {
            if v.is_empty() || !v.iter().any(|(_k, v)| v.is_object() || v.is_array()) {
                if v.iter().all(|(k, _v)| check_key(k)) {
                    return Ok(Value::Object(v));
                }
                let mut formatted_map = Map::<String, Value>::with_capacity(v.len());
                for (mut k, v) in v.into_iter() {
                    format_key(&mut k);
                    formatted_map.insert(k, v);
                }
                return Ok(Value::Object(formatted_map));
            }
            Value::Object(v)
        }
        _ => {
            return Err(anyhow::anyhow!("flatten value must be an object"));
        }
    };

    let mut flat = Map::<String, Value>::new();
    flatten_value(to_flatten, "".to_owned(), max_level, 0, &mut flat).map(|_x| Value::Object(flat))
}

/// Flattens the passed JSON value (`current`), whose path is `parent_key` and
/// its 0-based depth is `depth`.  The result is stored in the JSON object
/// `flattened`.
fn flatten_value(
    current: Value,
    parent_key: String,
    max_level: u32,
    depth: u32,
    flattened: &mut Map<String, Value>,
) -> Result<(), anyhow::Error> {
    match current {
        Value::Object(map) => {
            flatten_object(map, &parent_key, max_level, depth, flattened)?;
        }
        Value::Array(arr) => {
            flatten_array(arr, &parent_key, max_level, depth, flattened)?;
        }
        _ => {
            flattened.insert(parent_key, current);
        }
    }
    Ok(())
}

/// Flattens the passed object (`current`), whose path is `parent_key` and its
/// 0-based depth is `depth`.  The result is stored in the JSON object
/// `flattened`.
fn flatten_object(
    current: Map<String, Value>,
    parent_key: &str,
    max_level: u32,
    depth: u32,
    flattened: &mut Map<String, Value>,
) -> Result<(), anyhow::Error> {
    if current.is_empty() {
        return Ok(());
    }
    if max_level > 0 && depth >= max_level {
        let v = Value::String(Value::Object(current).to_string());
        flatten_value(v, parent_key.to_string(), max_level, depth, flattened)?;
        return Ok(());
    }
    for (mut k, v) in current.into_iter() {
        format_key(&mut k);
        let parent_key = if depth > 0 {
            format!("{}{}{}", parent_key, KEY_SEPARATOR, k)
        } else {
            k
        };
        flatten_value(v, parent_key, max_level, depth + 1, flattened)?;
    }
    Ok(())
}

/// Flattens the passed array (`current`), whose path is `parent_key` and its
/// 0-based depth is `depth`.  The result is stored in the JSON object
/// `flattened`.
fn flatten_array(
    current: Vec<Value>,
    parent_key: &str,
    max_level: u32,
    depth: u32,
    flattened: &mut Map<String, Value>,
) -> Result<(), anyhow::Error> {
    if current.is_empty() {
        return Ok(());
    }
    // for (i, obj) in current.iter().enumerate() {
    //     let parent_key = format!("{}{}{}", parent_key, KEY_SEPARATOR, i);
    //     flatten_value(obj, parent_key, depth + 1, flattened)?;
    // }
    let v = Value::String(Value::Array(current.to_vec()).to_string());
    flatten_value(v, parent_key.to_string(), max_level, depth, flattened)?;
    Ok(())
}

/// We need every character in the key to be lowercase alphanumeric or
/// underscore
pub fn format_key(key: &mut String) {
    if check_key(key) {
        return;
    }
    let mut key_chars = key.chars().collect::<Vec<_>>();
    for c in key_chars.iter_mut() {
        if c.is_lowercase() || c.is_numeric() {
            continue;
        } else if c.is_uppercase() {
            *c = c.to_lowercase().next().unwrap();
        } else {
            *c = '_';
        }
    }
    *key = key_chars.into_iter().collect::<String>();
}

fn check_key(key: &str) -> bool {
    key.chars()
        .all(|c| c.is_lowercase() || c.is_numeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_check_key_lowercase() {
        assert!(check_key("hello"));
    }

    #[test]
    fn test_check_key_numeric() {
        assert!(check_key("123"));
    }

    #[test]
    fn test_check_key_underscore() {
        assert!(check_key("my_key"));
    }

    #[test]
    fn test_check_key_mixed_case() {
        assert!(!check_key("Hello_World"));
    }

    #[test]
    fn test_check_key_special_characters() {
        assert!(!check_key("key!"));
    }

    #[test]
    fn object_with_plain_values() {
        let obj = json!({"int": 1, "float": 2.0, "str": "a", "bool": true, "null": null});
        assert_eq!(obj, flatten(obj.clone()).unwrap());
    }

    #[test]
    fn object_with_plain_values_with_format_key() {
        let obj = json!({"int": 1, "float": 2.0, "str": "a", "bool": true, "null": null});
        let obj2 = json!({"int": 1, "Float": 2.0, "str": "a", "bool": true, "null": null});
        assert_eq!(obj, flatten(obj2).unwrap());
    }

    /// Ensures that when using `ArrayFormatting::Plain` both arrays and objects
    /// are formatted properly.
    #[test]
    fn array_formatting_plain() {
        let obj = json!({"s": {"a": [1, 2.0, "b", null, true]}});
        assert_eq!(
            flatten(obj).unwrap(),
            json!({
                format!("s{k}a", k = KEY_SEPARATOR): "[1,2.0,\"b\",null,true]",
            })
        );
    }

    #[test]
    fn nested_single_key_value() {
        let obj = json!({"key": "value", "nested_key": {"key": "value"}});
        assert_eq!(
            flatten(obj).unwrap(),
            json!({"key": "value", "nested_key_key": "value"}),
        );
    }

    #[test]
    fn nested_multiple_key_value() {
        let obj = json!({"key": "value", "nested_key": {"key1": "value1", "key2": "value2"}});
        assert_eq!(
            flatten(obj).unwrap(),
            json!({"key": "value", "nested_key_key1": "value1", "nested_key_key2": "value2"}),
        );
    }

    #[test]
    fn complex_nested_struct() {
        let obj = json!({
            "simple_key": "simple_value",
            "key": [
                "value1",
                {"key": "value2"},
                {"nested_array": [
                    "nested1",
                    "nested2",
                    ["nested3", "nested4"]
                ]}
            ]
        });
        assert_eq!(
            flatten(obj).unwrap(),
            json!({"simple_key": "simple_value", "key": "[\"value1\",{\"key\":\"value2\"},{\"nested_array\":[\"nested1\",\"nested2\",[\"nested3\",\"nested4\"]]}]"}),
        );
    }

    // #[test]
    // fn overlapping_after_flattening_array() {
    //     let obj = json!({"key": ["value1", "value2"], "key_0": "Oopsy"});
    //     let res = flatten(&obj);
    //     assert!(res.is_err());
    //     match res {
    //         Err(err) => assert!(err.to_string().contains("key_0")),
    //         Ok(_) => panic!("This should have failed"),
    //     }
    // }

    /// Ensure that empty arrays are not present in the result
    #[test]
    fn empty_array() {
        let obj = json!({"key": []});
        assert_eq!(flatten(obj).unwrap(), json!({}));
    }

    /// Ensure that empty objects are not present in the result
    #[test]
    fn empty_object() {
        let obj = json!({"key": {}});
        assert_eq!(flatten(obj).unwrap(), json!({}));
    }

    #[test]
    fn empty_top_object() {
        let obj = json!({});
        assert_eq!(flatten(obj).unwrap(), json!({}));
    }

    /// Ensure that if all the end values of the JSON object are either `[]` or
    /// `{}` the flattened resulting object it's empty.
    #[test]
    fn empty_complex_object() {
        let obj = json!({"key": {"key2": {}, "key3": [[], {}, {"k": {}, "q": []}]}});
        assert_eq!(
            flatten(obj).unwrap(),
            json!({"key_key3": "[[],{},{\"k\":{},\"q\":[]}]"})
        );
    }

    #[test]
    fn nested_object_with_empty_array_and_string() {
        let obj = json!({"key": {"key2": [], "key3": "a"}});
        assert_eq!(flatten(obj).unwrap(), json!({"key_key3": "a"}));
    }

    #[test]
    fn nested_object_with_empty_object_and_string() {
        let obj = json!({"key": {"key2": {}, "key3": "a"}});
        assert_eq!(flatten(obj).unwrap(), json!({"key_key3": "a"}));
    }

    #[test]
    fn empty_string_as_key() {
        let obj = json!({"key": {"": "a"}});
        assert_eq!(flatten(obj).unwrap(), json!({"key_": "a"}));
    }

    #[test]
    fn empty_string_as_key_multiple_times() {
        let obj = json!({"key": {"": {"": {"": "a"}}}});
        assert_eq!(flatten(obj).unwrap(), json!({"key___": "a"}));
    }

    /// Flattening only makes sense for objects. Passing something else must
    /// return an informative error.
    #[test]
    fn first_level_must_be_an_object() {
        let integer = json!(3);
        let string = json!("");
        let boolean = json!(false);
        let null = json!(null);
        let array = json!([1, 2, 3]);

        for j in [integer, string, boolean, null, array].into_iter() {
            let res = flatten(j);
            match res {
                Err(_) => {} // Good
                Ok(_) => panic!("This should have failed"),
            }
        }
    }

    #[test]
    fn complex_array() {
        let obj = json!({"a": [1, [2, [3, 4], 5], 6]});
        assert_eq!(flatten(obj).unwrap(), json!({"a": "[1,[2,[3,4],5],6]"}));
    }

    #[test]
    fn complex_key_format() {
        let data = [
            (
                json!({"key": "value", "nested_key": {"key": "value", "foo": "bar"}}),
                json!({"key": "value", "nested_key_key": "value", "nested_key_foo": "bar"}),
            ),
            (
                json!({"key+bar": "value", "@nested_key": {"#key": "value", "&Foo": "Bar"}}),
                json!({"key_bar": "value", "_nested_key__key": "value", "_nested_key__foo": "Bar"}),
            ),
            (
                json!({"a": {"A.1": [1, [3, 4], 5], "A_2": 6}}),
                json!({"a_a_1": "[1,[3,4],5]", "a_a_2": 6}),
            ),
        ];
        for (input, expected) in data.into_iter() {
            assert_eq!(flatten(input).unwrap(), expected);
        }
    }

    #[test]
    fn test_flatten_json_complex() {
        let input = json!({
            "firstName": "John",
            "lastName": "Doe",
            "age": 25,
            "address": {
                "streetAddress": "123 Main St",
                "city": "Anytown",
                "state": "CA",
                "postalCode": "12345"
            },
            "phoneNumbers": [
                {
                    "type": "home",
                    "number": "555-555-1234"
                },
                {
                    "type": "work",
                    "number": "555-555-5678"
                }
            ]
        });

        let expected_output = json!({
            "firstname": "John",
            "lastname": "Doe",
            "age": 25,
            "address_streetaddress": "123 Main St",
            "address_city": "Anytown",
            "address_state": "CA",
            "address_postalcode": "12345",
            "phonenumbers":"[{\"number\":\"555-555-1234\",\"type\":\"home\"},{\"number\":\"555-555-5678\",\"type\":\"work\"}]"
        });

        let output = flatten(input).unwrap();
        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_flatten_with_level() {
        let input = json!({
            "firstName": "John",
            "lastName": "Doe",
            "age": 25,
            "info": {
                "address": {
                    "streetAddress": "123 Main St",
                    "city": "Anytown",
                    "state": "CA",
                    "postalCode": "12345",
                    "phoneNumbers": {
                        "type": "home",
                        "number": "555-555-1234"
                    }
                },
                "phoneNumbers": [
                    {
                        "type": "home",
                        "number": "555-555-1234"
                    },
                    {
                        "type": "work",
                        "number": "555-555-5678"
                    }
                ]
            }
        });

        let expected_output_level0 = json!({
            "firstname": "John",
            "lastname": "Doe",
            "age": 25,
            "info_address_streetaddress": "123 Main St",
            "info_address_city": "Anytown",
            "info_address_state": "CA",
            "info_address_postalcode": "12345",
            "info_address_phonenumbers_number": "555-555-1234",
            "info_address_phonenumbers_type": "home",
            "info_phonenumbers": "[{\"number\":\"555-555-1234\",\"type\":\"home\"},{\"number\":\"555-555-5678\",\"type\":\"work\"}]"
        });
        let expected_output_level1 = json!({
            "firstname": "John",
            "lastname": "Doe",
            "age": 25,
            "info": "{\"address\":{\"city\":\"Anytown\",\"phoneNumbers\":{\"number\":\"555-555-1234\",\"type\":\"home\"},\"postalCode\":\"12345\",\"state\":\"CA\",\"streetAddress\":\"123 Main St\"},\"phoneNumbers\":[{\"number\":\"555-555-1234\",\"type\":\"home\"},{\"number\":\"555-555-5678\",\"type\":\"work\"}]}"
        });
        let expected_output_level2 = json!({
            "firstname": "John",
            "lastname": "Doe",
            "age": 25,
            "info_address": "{\"city\":\"Anytown\",\"phoneNumbers\":{\"number\":\"555-555-1234\",\"type\":\"home\"},\"postalCode\":\"12345\",\"state\":\"CA\",\"streetAddress\":\"123 Main St\"}",
            "info_phonenumbers": "[{\"number\":\"555-555-1234\",\"type\":\"home\"},{\"number\":\"555-555-5678\",\"type\":\"work\"}]"
        });
        let expected_output_level3 = json!({
            "firstname": "John",
            "lastname": "Doe",
            "age": 25,
            "info_address_streetaddress": "123 Main St",
            "info_address_city": "Anytown",
            "info_address_state": "CA",
            "info_address_postalcode": "12345",
            "info_address_phonenumbers": "{\"number\":\"555-555-1234\",\"type\":\"home\"}",
            "info_phonenumbers": "[{\"number\":\"555-555-1234\",\"type\":\"home\"},{\"number\":\"555-555-5678\",\"type\":\"work\"}]"
        });
        let expected_output_level4 = json!({
            "firstname": "John",
            "lastname": "Doe",
            "age": 25,
            "info_address_streetaddress": "123 Main St",
            "info_address_city": "Anytown",
            "info_address_state": "CA",
            "info_address_postalcode": "12345",
            "info_address_phonenumbers_number": "555-555-1234",
            "info_address_phonenumbers_type": "home",
            "info_phonenumbers": "[{\"number\":\"555-555-1234\",\"type\":\"home\"},{\"number\":\"555-555-5678\",\"type\":\"work\"}]"
        });

        let output = flatten_with_level(input.clone(), 0).unwrap();
        assert_eq!(output, expected_output_level0);
        let output = flatten_with_level(input.clone(), 1).unwrap();
        assert_eq!(output, expected_output_level1);
        let output = flatten_with_level(input.clone(), 2).unwrap();
        assert_eq!(output, expected_output_level2);
        let output = flatten_with_level(input.clone(), 3).unwrap();
        assert_eq!(output, expected_output_level3);
        let output = flatten_with_level(input.clone(), 4).unwrap();
        assert_eq!(output, expected_output_level4);
        let output = flatten_with_level(input, 5).unwrap();
        assert_eq!(output, expected_output_level4);
    }
}

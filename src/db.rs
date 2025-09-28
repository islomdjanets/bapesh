use std::default;

// use anyhow::Ok;
use sqlx::{pool, postgres::{PgPoolOptions, PgQueryResult, PgRow, PgTypeInfo, PgValueRef}, Connection, Decode, Postgres, Row, TypeInfo, ValueRef};
// use sqlx::postgres::type_info::PgTypeInfo;
use sqlx::Column;
use crate::json::JSON;
// use std::borrow::Cow;

use serde_json::json;

pub type StdError = Box<dyn std::error::Error + Send + Sync>;
pub type Pool = sqlx::Pool<sqlx::Postgres>; 

pub async fn connect() -> Result<Pool, StdError> {
    // let url = "";
    // let pool = sqlx::postgres::PgPool::connect(url).await?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    Ok(pool)
}

pub async fn get_sql_type(r#type: &str, is_array: bool) -> String {
    let std_type = match r#type {
        "Struct" | "struct" | "json" | "JSON" => "JSONB", // Use JSONB for arbitrary structs

        "string" | "str" => "TEXT",
        "int" => "INTEGER",
        "float" => "REAL",
        "bool" => "BOOLEAN",

        "u8" | "u16" | "i8" | "i16" => "SMALLINT",
        "u32" | "i32" => "INTEGER",
        "u64" | "usize" | "i64" | "isize" => "BIGINT",

        "f32" => "REAL",
        "f64" => "DOUBLE PRECISION",

        "Timestamp" | "timestamp" => "TIMESTAMP WITH TIME ZONE",
        "DateTime" | "datetime" => "TIMESTAMP WITHOUT TIME ZONE",
        "Date" | "date" => "DATE",
        "Time" | "time" => "TIME",

        // "Vector2" => "Vector2",
        // "Vector3" => "Vector3",
        // "Vector4" => "Vector4",
        // "Color" => "Color", // Assuming Color is a 4-component RGBA color
        //-- Insert data
        //INSERT INTO objects (position) VALUES (ROW(10.5, 20.3)::vector2);

        "Vector2" | "vector2" | "vec2" | "Vec2" => if is_array { "REAL" } else { "REAL[2]" },
        "Vector3" | "vector3" | "vec3" | "Vec3" => if is_array { "REAL" } else { "REAL[3]" },
        "Vector4" | "vector4" | "vec4" | "Vec4" => if is_array { "REAL" } else { "REAL[4]" },
        "Color" | "color" => if is_array { "SMALLINT[]" } else { "SMALLINT" }, // Assuming Color is a 4-component RGBA color
        //-- Insert data
        //INSERT INTO objects (position) VALUES (ARRAY[10.5, 20.3]::REAL[]);

        _ => "TEXT", // Default to TEXT for unknown types 
    };

    // if std_type == "TEXT" {
    //     let exists = is_custom_type_exists(r#type, pool).await;
    //     if exists.is_ok() {
    //         return r#type.to_string(); // If we can't check, return the standard type
    //     }
    //     // If it's a reference type, we assume it's a foreign key
    // }

    std_type.to_string() // or whatever type you use for foreign keys
}

pub async fn is_table_exists(name: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<bool, StdError> {
    let query = "SELECT EXISTS (
        SELECT FROM information_schema.tables 
        WHERE table_name = $1
    )";

    let row = sqlx::query(query)
        .bind(name)
        .fetch_one(pool)
        .await?;

    let exists: bool = row.get(0);
    Ok(exists)
}

pub async fn remove_from_table(name: &str, id: i64, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("DELETE FROM {} WHERE id = $1", name);
    sqlx::query(query)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_from_table(name: &str, id: i64, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<Option<JSON>, StdError> {
    let query = &format!("SELECT * FROM {} WHERE id = $1", name);
    let row = sqlx::query(query)
        .bind(id)
        // .fetch_optional(pool)
        .fetch_one(pool)
        // .execute(pool)
        .await;

    // if row.is_err() {
    //     println!("Error fetching row: {:?}", row.err());
    //     return Ok(None);
    // }

    // let row = row.unwrap();


    if let Ok(row) = row {
        // let json: JSON = row.try_get("data")?; // Assuming the column is named 'data'
        // Ok(Some(json))
        let json = row_to_json(&row);
        if json.is_none() {
            println!("Error converting row to JSON");
            return Ok(None);
        }
        return Ok(Some(json.unwrap()));

    } else {
        println!("No row found with id: {} {}", id, row.err().unwrap());
        Ok(None)
    }
}

pub fn row_to_json(row: &PgRow) -> Option<JSON> {
    println!("Row: {:?}", row);
    let mut obj = serde_json::Map::new();
    for column in row.columns() {
        let col_name = column.name();
        // println!("Column: {}", col_name);

        // Get raw value to inspect type without decoding
        let raw_value = row.try_get_raw(col_name);
        match raw_value {
            Ok(raw) => {
                if raw.is_null() {
                    obj.insert(col_name.to_string(), JSON::Null);
                    continue;
                }

                // Get type info
                let type_info = raw.type_info();
                let type_oid = type_info.oid().map(|oid| oid.0 as u32);  // OID via public oid() method
                let type_name = type_info.name();  // String like "int8", "text[]", "timestamptz"

                println!("Decoding type: OID={} Name={}", type_oid.unwrap_or(0), type_name);

                // Dispatch based on OID (primary) or fallback to name matching
                let value = match type_oid {
                    // JSON/JSONB OIDs
                    Some(114) | Some(3802) => {
                        <JSON as Decode<Postgres>>::decode(raw).unwrap_or(JSON::Null)
                    }
                    // INTEGER
                    Some(23) => {
                        let num: i32 = <i32 as Decode<Postgres>>::decode(raw).unwrap_or(0);
                        json!(num)
                    }
                    // TIMESTAMP
                    Some(1114) => {
                        // Requires sqlx "chrono" feature
                        use chrono::{NaiveDateTime};
                        match <NaiveDateTime as Decode<Postgres>>::decode(raw) {
                            Ok(dt) => json!(dt.format("%Y-%m-%dT%H:%M:%S").to_string()),
                            Err(_) => json!(null),
                        }
                    }
                    // BIGINT/INT8 OID
                    Some(20) => {
                        let num: i64 = <i64 as Decode<Postgres>>::decode(raw).unwrap_or(0);
                        json!(num)
                    }
                    // u64/UINT8 OID
                    Some(1700) | Some(701) => {
                        let num: f64 = <f64 as Decode<Postgres>>::decode(raw).unwrap_or(0.0);
                        json!(num as u64)
                    }
                    // FLOAT4/REAL OID
                    Some(700) => {
                        let num: f32 = <f32 as Decode<Postgres>>::decode(raw).unwrap_or(0.0);
                        json!(num)
                    }
                    //REAL[]
                    // REAL[][]
                    // Some(1021) => {
                    //     println!("Decoding 2D float array");
                    //     match <Vec<Vec<f32>> as Decode<Postgres>>::decode(raw.clone()) {
                    //         Ok(arr) => json!(arr),
                    //         Err(_) => {
                    //             let arr_str: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("[]");
                    //             serde_json::from_str(arr_str).unwrap_or_else(|_| json!([]))
                    //         }
                    //     }
                    // }
                    // Array types (examples: TEXT[]=1009, INT4[]=1007, etc.)
                    Some(1009) | Some(1000) | Some(1007) | Some(1014) | Some(1015) | Some(1016) | Some(1005) | Some(1020) | Some(1021) => {
                        println!("Decoding array type");
                        // Decode to Vec<String> (native for text[]; adjust for other elem types)
                        // For empty: vec![], for non-empty: vec!["item1", "item2"]
                        if type_oid == Some(1021) || type_oid == Some(1020) {
                            // REAL[]
                            // FLOAT4[] rust type is f32
                            match <Vec<f32> as Decode<Postgres>>::decode(raw.clone()) {
                                Ok(arr) => json!(arr),
                                Err(_) => {
                                    println!("Falling back to text decode for REAL[], {} {}", type_oid.unwrap_or(0), type_name);
                                    let arr_str: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("[]");
                                    serde_json::from_str(arr_str).unwrap_or_else(|_| json!([]))
                                }
                            }
                        }
                        else if type_oid == Some(1016) {
                            // BIGINT[]
                            match <Vec<i64> as Decode<Postgres>>::decode(raw.clone()) {
                                Ok(arr) => json!(arr),
                                Err(_) => {
                                    let arr_str: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("[]");
                                    serde_json::from_str(arr_str).unwrap_or_else(|_| json!([]))
                                }
                            }
                        } else if type_oid == Some(1007) {
                            // INT4[]
                            match <Vec<i32> as Decode<Postgres>>::decode(raw.clone()) {
                                Ok(arr) => json!(arr),
                                Err(_) => {
                                    let arr_str: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("[]");
                                    serde_json::from_str(arr_str).unwrap_or_else(|_| json!([]))
                                }
                            }
                        } else if type_oid == Some(1005) {

                            // SMALLINT[]
                            match <Vec<i16> as Decode<Postgres>>::decode(raw.clone()) {
                                Ok(arr) => json!(arr),
                                Err(_) => {
                                    let arr_str: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("[]");
                                    serde_json::from_str(arr_str).unwrap_or_else(|_| json!([]))
                                }
                            }
                        } else if type_oid == Some(1000) {
                            // BOOL[]
                            match <Vec<bool> as Decode<Postgres>>::decode(raw.clone()) {
                                Ok(arr) => json!(arr),
                                Err(_) => {
                                    let arr_str: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("[]");
                                    serde_json::from_str(arr_str).unwrap_or_else(|_| json!([]))
                                }
                            }
                        } else {
                            match <Vec<String> as Decode<Postgres>>::decode(raw.clone()) {
                                Ok(arr) => json!(arr),  // Converts Vec<String> to JSON array
                                Err(_) => {
                                    // Fallback: text decode + parse (for edge cases)
                                    let arr_str: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("[]");
                                    serde_json::from_str(arr_str).unwrap_or_else(|_| json!([]))
                                }
                            }
                        }
                    }
                    // TIMESTAMPTZ OID (1184)
                    Some(1184) => {
                        // Requires sqlx "chrono" feature
                        use chrono::{DateTime, Utc};
                        match <DateTime<Utc> as Decode<Postgres>>::decode(raw) {
                            Ok(dt) => json!(dt.to_rfc3339()),
                            Err(_) => json!(null),
                        }
                    }
                    // TEXT/VARCHAR OIDs (25, 1043)
                    Some(25) | Some(1043) => {
                        let s: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("");
                        json!(s)
                    }
                    // Fallback: Use type_name for unknown OIDs
                    _ => {
                        println!("Unknown OID: {}, falling back to name match: {}", type_oid.unwrap_or(0), type_name);
                        match type_name.to_lowercase().as_str() {
                            "int8" | "bigint" => {
                                let num: i64 = <i64 as Decode<Postgres>>::decode(raw).unwrap_or(0);
                                json!(num)
                            }
                            "timestamp with time zone" | "timestamptz" => {
                                use chrono::{DateTime, Utc};
                                match <DateTime<Utc> as Decode<Postgres>>::decode(raw) {
                                    Ok(dt) => json!(dt.to_rfc3339()),
                                    Err(_) => json!(null),
                                }
                            }
                            "text[]" | "character varying[]" | "integer[]" | "int8[]" | "bigint[]" => {
                                // Same Vec<String> decode for string arrays; for int[] use Vec<i32>
                                match <Vec<String> as Decode<Postgres>>::decode(raw.clone()) {
                                    Ok(arr) => json!(arr),
                                    Err(_) => {
                                        let arr_str: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("[]");
                                        serde_json::from_str(arr_str).unwrap_or_else(|_| json!([]))
                                    }
                                }
                            }
                            "json" | "jsonb" => {
                                <JSON as Decode<Postgres>>::decode(raw).unwrap_or(JSON::Null)
                            }
                            _ => {
                                // Ultimate fallback: Try as string
                                println!("Fallback to string decode for type name: {}", type_name);
                                let s: &str = <&str as Decode<Postgres>>::decode(raw).unwrap_or("null");
                                json!(s)
                            }
                        }
                    }
                };
                obj.insert(col_name.to_string(), value);
            }
            Err(e) => {
                println!("Error getting raw column {}: {:?}", col_name, e);
                obj.insert(col_name.to_string(), JSON::Null);
            }
        }
    }
    // // get values from row
    // let mut obj = serde_json::Map::new();
    // for column in row.columns() {
    //     let col_name = column.name();
    //     println!("Column: {}", col_name);
    //     let value: Result<JSON, _> = row.try_get_unchecked(col_name);
    //     if let Ok(value) = value {
    //         obj.insert(col_name.to_string(), value);
    //     } else {
    //         println!("Error getting column {}: {:?}", col_name, value.err());
    //         obj.insert(col_name.to_string(), JSON::Null);
    //     }
    // }

    println!("values: {:?}", obj);
    Some(JSON::Object(obj))
}

pub async fn insert_into_table_and_return_id(name: &str, values: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<i64, StdError> {
    let (keys, values) = generate_values(values);

    let query = &format!("INSERT INTO {} ({}) VALUES ({}) RETURNING id", name, keys, values);
    println!("Insert query: {}", query);

    let query = sqlx::query(query);
    let id: i64 = query
        .fetch_one(pool)
        .await?
        .get("id");

    Ok(id)
}

pub async fn insert_into_table_and_return(name: &str, values: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<JSON, StdError> {
    let (keys, values) = generate_values(values);

    println!("Inserting into table: {}", name);
    println!("Keys: {}", keys);
    println!("Values: {}", values);

    // use RETURNING * to get the inserted row
    // or RETURNING id to get the inserted id
    let query = &format!("INSERT INTO {} ({}) VALUES ({}) RETURNING *", name, keys, values);
    println!("Insert query: {}", query);

    let query = sqlx::query(query);
    let row = query
        .fetch_one(pool)
        .await?;

    // if result.is_err() {
    //     println!("Error inserting into table: {:?}", result.as_ref().err());
    //     return Err(Box::new(result.unwrap_err()) as StdError);
    // }

    // Manually convert the row to JSON
    let mut obj = serde_json::Map::new();
    for column in row.columns() {
        let col_name = column.name();
        let value: Result<JSON, _> = row.try_get_unchecked(col_name);
        if let Ok(value) = value {
            obj.insert(col_name.to_string(), value);
        } else {
            obj.insert(col_name.to_string(), JSON::Null);
        }
    }

    Ok(JSON::Object(obj))
}

pub async fn insert_into_table(name: &str, values: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let (keys, values) = generate_values(values);

    println!("Inserting into table: {}", name);
    println!("Keys: {}", keys);
    println!("Values: {}", values);

    let query = &format!("INSERT INTO {} ({}) VALUES ({})", name, keys, values);
    println!("Insert query: {}", query);

    let result = sqlx::query(query)
        .execute(pool)
        .await;

    if result.is_err() {
        println!("Error inserting into table: {:?}", result.as_ref().err());
        return Err(Box::new(result.unwrap_err()) as StdError);
    }

    println!("Rows affected: {}", result.unwrap().rows_affected());

    Ok(())
}

pub fn generate_values(json: &JSON) -> (String, String) {
    let mut keys = String::new();
    let mut values = String::new();
    if json.is_null() {
        return (keys, values);
    }

    if let JSON::Object(obj) = json {
        for (key, value) in obj.iter() {
            // let key = if key.starts_with('!') {
            //     // PRIMARY KEY
            //     key.trim_start_matches('!').to_string()
            // } else {
            //     key.to_string()
            // };

            // println!("Processing key: {}, value: {:?}", key, value);
            // let value = value.as_str().unwrap_or("NULL");
            // result.push_str(&format!("{}: {}, ", key, value));

            keys.push_str(&format!("{}, ", key));

            if value.is_string() {
                let s = value.as_str().unwrap_or("");
                values.push_str(&format!("'{}', ", s.replace("'", "''"))); // Escape single quotes
            } else if value.is_null() {
                values.push_str("NULL, ");
            } else {
                values.push_str(&format!("{}, ", value));
            }

            // values.push_str(&format!("{}, ", value));

        }
        keys = keys.trim_end_matches(", ").to_string();
        values = values.trim_end_matches(", ").to_string();
    }

    (keys, values)
}

pub async fn generate_properties(schema: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> String {
    let mut properties = String::new();
    if schema.is_null() {
        return properties;
    }

    if let JSON::Object(obj) = schema {
        for (key, mut value) in obj.iter() {

            // println!("Processing key: {}, value: {:?}", key, value);

            let mut default_value = "NULL".to_string();
            // println!("Key: {}, Value: {:?}", key, value);
            if value.is_object() {
                let info = value.as_object().unwrap();
                value = info.get("type").unwrap_or(value);
                let description = info.get("description").unwrap_or(&JSON::Null);
                let default = info.get("default").unwrap_or(&JSON::Null);
                default_value = match default {
                    JSON::String(s) => format!("'{}'", s.replace("'", "''")),
                    JSON::Number(n) => format!("{}", n),
                    JSON::Bool(b) => format!("{}", b),
                    JSON::Null => "NULL".to_string(),
                    JSON::Array(arr) => {
                        let arr_str: Vec<String> = arr.iter().map(|v| {
                            if v.is_string() {
                                let value = v.as_str().unwrap_or("");
                                if value.starts_with("Vector") {
                                    let dims = value.chars().nth(6).unwrap().to_digit(10).unwrap_or(0); // Get the character after "Vector"
                                    // println!("Value is a Vector: {} with dimensions of {}", value, dims);
                                    // let dimensions = dims;
                                    // println!("Vector's dimensions for key {}: {}", key, dimensions);
                                    // retrieve values from inside Vector3(these are comma separated)
                                    let value = &value[8..value.len()-1]; // Get inside the parentheses
                                    let values = format!("{}", value);
                                    // let values = format!("'{{{}}}'", "0.0".repeat(dimensions as usize).chars().collect::<Vec<char>>().chunks(2).map(|c| c.iter().collect::<String>()).collect::<Vec<String>>().join(", "));
                                    println!("Vector default value for key {}: {}", key, values);
                                    values
                                }
                                else if value.starts_with("Color") {
                                    let value = &value[7..value.len()-1]; // Get inside the parentheses
                                    let values = format!("{}", value);
                                    println!("Color default value for key {}: {}", key, values);
                                    values
                                }
                                 else {
                                    format!("\"{}\"", value.replace("'", "\""))
                                }
                            } else if v.is_null() {
                                "NULL".to_string()
                            } else {
                                format!("{}", v)
                            }
                        }).collect();
                        println!("Array default value for key {}: {:?}", key, arr_str);
                        format!("{{{}}}", arr_str.join(", "))
                    },
                    JSON::Object(obj) => {
                        // Complex default values not handled
                        println!("Warning: Complex default value for key {} not handled", key);
                        "NULL".to_string()
                    },
                };
                println!("Default value for {}: {}", key, default_value);
                // if !default.is_null() {
                //     if default.is_string() {
                //         default_value = format!("'{}'", default.as_str().unwrap_or("").replace("'", "''"));
                //     } else {
                //         default_value = format!("{}", default);
                //     }
                //     println!("Default value for {}: {}", key, default_value);
                // }

                println!("Property: {}, Type: {}, Description: {}, Default: {}", key, value, description, default);

                // println!("Value is an object. Type: {:?}", value);
            }
            let mut is_primary_key = false;
            let key = if key.starts_with('!') {
                // PRIMARY KEY
                is_primary_key = true;
                key.trim_start_matches('!').to_string()
            } else {
                key.to_string()
            };

            let value = value.as_str().unwrap_or("string");

            let default_type = "BIGINT"; // Default type for references
            if value.starts_with('[') {
                // Array
                let inner_type = value.trim_start_matches('[').trim_end_matches(']');
                if inner_type.starts_with('&') {
                    // Reference type
                    let table = inner_type.trim_start_matches('&');
                    properties.push_str(&format!("{} {}[] DEFAULT '{{}}', ", key, default_type));
                    continue;
                } else if inner_type.contains("::") {
                    // Reference type with property
                    let (table, property) = {
                        let parts: Vec<&str> = inner_type.split("::").collect();
                        (parts[0], parts[1])
                    };
                    let sql_type = default_type; // must get type from schema
                    properties.push_str(&format!("{} {}[] DEFAULT '{{}}', ", key, sql_type));
                    continue;
                }

                let sql_type = get_sql_type(inner_type, true).await;
                if default_value == "NULL" {
                    default_value = "{}".to_string(); // Empty array
                }
                //  else if !default_value.starts_with('{') {
                //     default_value = format!("{{{}}}", default_value); // Wrap in array braces if not already
                // }
                properties.push_str(&format!("{} {}[] DEFAULT '{}', ", key, sql_type, default_value));
                continue;
                // array type

            }
            else if value.starts_with('{') {
                // Map
                // println!("Value is a Map or Set: {}", value);
                let inner_types = value.trim_start_matches('{').trim_end_matches('}');
                if !inner_types.contains(',') {
                    // Set
                    let inner_type = inner_types.trim();
                    let is_ref = inner_type.starts_with('&');

                    // println!("Set: {}", inner_type);

                    let sql_type = get_sql_type(inner_type, false).await;
                    properties.push_str(&format!("{} {}[] DEFAULT '{{}}', ", key, sql_type));
                    continue;
                }
                else {
                    let inner_types: Vec<&str> = inner_types.split(',').collect();
                    
                    let inner_key = inner_types[0].trim();
                    let inner_value = inner_types[1].trim();
                    let is_ref = inner_value.starts_with('&');

                    // println!("Map: {} , {}", inner_key, inner_value);

                    // Map type

                    // let sql_type = get_sql_type(value);

                    let is_simple_type = inner_value == "string" || inner_value == "int" || inner_value == "float" || inner_value == "bool";
                    let map_type = if is_simple_type {
                        format!("{} HSTORE DEFAULT ''::HSTORE, ", key)
                    } else {
                        // let sql_type = get_sql_type(inner_value);
                        format!("{} JSONB DEFAULT '{{}}'::JSONB, ", key)
                    };
                    properties.push_str(&map_type);
                    continue;
                }
            }

            let is_nullable = key.starts_with('?');
            
            if value.starts_with('&') {
                // Reference type
                let table = value.trim_start_matches('&');
                let sql_type = default_type; // must get type from schema
                // properties.push_str(&format!("{} {} REFERENCES {}(id) {}, ", key, sql_type, table, if is_nullable { "" } else { "NOT NULL" }));
                properties.push_str(&format!("{} {} REFERENCES {}(id), ", key, sql_type, table));
            } else if value.contains("::") {
                // Reference type
                // let ref_name = value.trim_start_matches('&');
                let (table, property) = {
                    let parts: Vec<&str> = value.split("::").collect();
                    (parts[0], parts[1])
                };
                // let sql_type = get_sql_type(v, pool).await;
                let sql_type = default_type; // must get type from schema

                properties.push_str(&format!("{} {} REFERENCES {}({}), ", key, sql_type, table, property));
            } else {
                let sql_type = get_sql_type(value, false).await;
                let default_value = get_default_value(&sql_type);
                // properties.push_str(&format!("{} {} DEFAULT {} {} {}, ", key, sql_type, default_value, if is_nullable { "" } else { "NOT NULL" }, if is_primary_key { "PRIMARY KEY" } else { "" }));

                if is_primary_key {
                    // properties.push_str(&format!("{} {} BIGSERIAL PRIMARY KEY, ", key, sql_type));
                    properties.push_str(&format!("{} BIGSERIAL PRIMARY KEY, ", key));
                }
                else {
                    properties.push_str(&format!("{} {} DEFAULT {}, ", key, sql_type, default_value));
                }
            }
        }
    }
    properties = properties.trim_end_matches(", ").to_string();

    return properties;
}

pub fn get_default_value(r#type: &str) -> String {
    // println!("Getting default value for type: {}", r#type);
    match r#type {
        "TEXT" => "''",
        "BIGINT" | "INTEGER" | "int" | "u8" | "u16" | "i8" | "i16" | "u32" | "i32" | "u64" | "usize" | "i64" | "isize" => "0",
        "REAL" | "float" | "f32" | "f64" => "0.0",
        "BOOLEAN" | "bool" => "FALSE",

        "TIMESTAMP WITH TIME ZONE" => "CURRENT_TIMESTAMP",
        "TIMESTAMP WITHOUT TIME ZONE" => "CURRENT_TIMESTAMP",
        "DATE" => "CURRENT_DATE",
        "TIME" => "CURRENT_TIME",
        
        "REAL[2]" => "'{0.0, 0.0}'",
        "REAL[3]" => "'{0.0, 0.0, 0.0}'",
        "REAL[4]" => "'{0.0, 0.0, 0.0, 0.0}'",
        "SMALLINT[4]" => "'{0, 0, 0, 0}'", // Assuming Color is a 4-component RGBA color

        "ARRAY" => "'{}'",
        "JSONB" => "'{}'",
        "HSTORE" => "''",

        _ => "NULL", // Default to NULL for unknown types
    }.to_string()
}

pub async fn create_table(name: &str, schema: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let properties = generate_properties(schema, pool).await;
    println!("Table: {} Properties: {}", name, properties);

    let query = &format!("CREATE TABLE {} ({});", name, properties);
    // println!("Create table query: {}", query);

    sqlx::query(query)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn create_table_if_not_exists(name: &str, schema: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let properties = generate_properties(schema, pool).await;

    let query = &format!("CREATE TABLE IF NOT EXISTS {} ({})", name, properties);

            // id SERIAL PRIMARY KEY,
            // name TEXT NOT NULL,
            // description TEXT,
            // logo TEXT

    sqlx::query(query)
        .execute(pool)
        .await?;

    // sqlx::query(
    //     "CREATE TABLE IF NOT EXISTS decors (
    //         id SERIAL PRIMARY KEY,
    //         name TEXT NOT NULL,
    //         description TEXT,
    //         geometry TEXT,
    //         material TEXT,
    //         type TEXT,
    //         prestige_per_hour INTEGER,
    //         bundle INTEGER REFERENCES bundles(id),
    //         price NUMERIC(10, 2),
    //         cover TEXT
    //     )",
    // )
    // .execute(pool)
    // .await?;

    Ok(())
}

pub async fn delete_table(name: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("DROP TABLE IF EXISTS {}", name);
    sqlx::query(query)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_database(name: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("CREATE DATABASE {}", name);
    sqlx::query(query)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_database(name: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("DROP DATABASE IF EXISTS {}", name);
    sqlx::query(query)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn is_extension_exists(name: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<bool, StdError> {
    let query = "SELECT EXISTS (
        SELECT 1 FROM pg_extension WHERE extname = $1
    )";

    let row = sqlx::query(query)
        .bind(name)
        .fetch_one(pool)
        .await?;

    let exists: bool = row.get(0);
    Ok(exists)
}

pub async fn enable_extension(name: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    sqlx::query(&format!("CREATE EXTENSION IF NOT EXISTS {}", name))
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_type(name: &str, schema: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let properties = generate_properties(schema, pool).await;

    let query = &format!("CREATE TYPE IF NOT EXISTS {} AS ({})", name, properties);

    sqlx::query(query)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn create_enum(name: &str, variants: &[&str], pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let variants_str = variants.join(", ");
    let query = &format!("CREATE TYPE IF NOT EXISTS {} AS ENUM ({})", name, variants_str);

    sqlx::query(query)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_type(name: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("DROP TYPE IF EXISTS {}", name);
    sqlx::query(query)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn is_custom_type_exists(name: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<bool, StdError> {
    let query = "SELECT EXISTS (
        SELECT 1 FROM pg_type WHERE typname = $1
    )";

    let row = sqlx::query(query)
        .bind(name)
        .fetch_one(pool)
        .await?;

    let exists: bool = row.get(0);
    Ok(exists)
}

pub async fn get_tables(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<Vec<JSON>, StdError> {
    let query = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'";

    let rows = sqlx::query(query)
        .fetch_all(pool)
        .await?;

    let mut tables = Vec::new();
    for row in rows {
        let table_name: String = row.get("table_name");
        tables.push(JSON::String(table_name));
    }

    Ok(tables)
}

pub async fn update_jsonb_array_by_key(table: &str, map_name: &str, id: i64, key: &str, value: &str, index: Option<i32>, create_if_not_exists: bool, pool: &Pool) -> Result<(), StdError> {
    let value = format!("[{}]", value); // Wrap value in array brackets
    println!("Value to insert: {}", value);

    let query = {
        // if !index.is_none() {
        //     format!("UPDATE {} SET {} = jsonb_set({}, '{{{}, {}}}', $1::jsonb, {}) WHERE id = $2",
        //         table, map_name, map_name, key, index.unwrap(), create_if_not_exists)
        // } else {
        //     format!("UPDATE {} SET {} = jsonb_set({}, '{{{}}}', ({}->'{}' || $1::jsonb), {}) WHERE id = $2",
        //         table, map_name, map_name, key, map_name, key, create_if_not_exists)
        // }
        if !index.is_none() {
            format!("UPDATE {} SET {} = jsonb_set({}, '{{{}, {}}}', $1::jsonb, {}) WHERE id = $2",
                table, map_name, map_name, key, index.unwrap(), create_if_not_exists)
        } else {
            format!("UPDATE {} SET {} = jsonb_set(COALESCE({}, '{{}}'::jsonb), '{{{}}}', (COALESCE({}->'{}', '[]'::jsonb) || $1::jsonb), {}) WHERE id = $2",
                table, map_name, map_name, key, map_name, key, create_if_not_exists)
        }
    };
    println!("Update JSONB array by key query: {}", query);

    sqlx::query(&query)

        .bind(value)
        .bind(id)

        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update(table: &str, key: &str, id: i64, value: &str, pool: &Pool, cast: &str) -> Result<(), StdError> {
    let query = &format!("UPDATE {} SET {} = $1::{} WHERE id = $2", table, key, cast);

    println!("Update query: {}", query);
    sqlx::query(query)

        .bind(value)
        .bind(id)

        .execute(pool)
        .await?;
    Ok(())
}

pub async fn remove_from_set(table: &str, column: &str, id: i64, value: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("UPDATE {} SET {} = array_remove({}, $1) WHERE id = $2", table, column, column);
    sqlx::query(query)
        .bind(value)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn remove_from_array(table: &str, column: &str, id: i64, index: f32, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {

    if index == 0.0 {
        return Ok(()); // No-op for index 0
    }

    if index == 1.0 {
        // Remove first element
        let query = &format!("UPDATE {} SET {} = {}[2:array_length({}, 1)] WHERE id = $1", table, column, column, column);
        sqlx::query(query)
            .bind(id)
            .execute(pool)
            .await?;

        return Ok(());
    }
    if index == -1.0 {
        // Remove last element
        let query = &format!("UPDATE {} SET {} = {}[1:array_length({}, 1)-1] WHERE id = $1", table, column, column, column);
        sqlx::query(query)
            .bind(id)
            .execute(pool)
            .await?;

        return Ok(());
    }
    let start = index - 1.0;
    let end = index + 1.0;
    let query = &format!("UPDATE {} SET {} = array_cat({}[{}:{}], {}[{}:{}]) WHERE id = $1", table, column, column, start as i32, start as i32, column, end as i32, end as i32);
    println!("Remove for Array Query: {}", query);
    // let query = &format!("UPDATE {} SET {} = array_cat({}, $1) WHERE id = $2", table, column, column);
    sqlx::query(query)
        .bind(index)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn empty_array(table: &str, column: &str, id: i64, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("UPDATE {} SET {} = '{{}}' WHERE id = $1", table, column);
    sqlx::query(query)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn add_to_array(table: &str, column: &str, id: i64, value: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("UPDATE {} SET {} = array_append({}, $1) WHERE id = $2", table, column, column);
    sqlx::query(query)
        .bind(value)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn add_number_to_array(table: &str, column: &str, id: i64, value: i64, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("UPDATE {} SET {} = array_append({}, $1::text::bigint) WHERE id = $2", table, column, column);
    sqlx::query(query)
        .bind(value)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn empty_jsonb(table: &str, column: &str, id: i64, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("UPDATE {} SET {} = '{{}}'::JSONB WHERE id = $1", table, column);
    sqlx::query(query)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn empty_hstore(table: &str, column: &str, id: i64, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("UPDATE {} SET {} = ''::HSTORE WHERE id = $1", table, column);
    sqlx::query(query)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn remove_jsonb_by_key(table: &str, column: &str, id: i64, key: &JSON, pool: &Pool) -> Result<(), StdError> {
    let query = &format!(
        r#"UPDATE {}
        SET {} = {} - $1
        WHERE id = $2"#,
        table, column, column
    );

    println!("Remove JSONB by key query: {}", query);

    sqlx::query(query)
        .bind(key)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn remove_hstore_by_key(table: &str, column: &str, id: i64, key: &str, pool: &Pool) -> Result<(), StdError> {
    let query = &format!(
        r#"UPDATE {}
        SET {} = delete({}, $1)
        WHERE id = $2"#,
        table, column, column
    );

    println!("Remove HSTORE by key query: {}", query);

    sqlx::query(query)
        .bind(key)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_hstore_by_key(
    table: &str, column: &str, id: i64,
    key: &str, value: &JSON, create_if_not_exists: bool,
    pool: &Pool,// &sqlx::Pool<sqlx::Postgres>
) -> Result<(), StdError> {

    let query = &format!(
        r#"UPDATE {}
        SET {} = {} || hstore($1, $2::hstore)
        WHERE id = $3"#,
        table, column, column
    );

    println!("Update HSTORE by key query: {}", query);

    sqlx::query(query)
        .bind(key)
        .bind(value)
        .bind(id)

        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_jsonb_by_key(
    table: &str, column: &str, id: i64,
    key: &str, value: &JSON, create_if_not_exists: bool,
    pool: &Pool,// &sqlx::Pool<sqlx::Postgres>
) -> Result<(), StdError> {

    let query = &format!(
        r#"UPDATE {}
        SET {} = jsonb_set({}, '{{{}}}', $2::jsonb, $3)
        WHERE id = $4"#,
        table, column, column, key,
    );

    println!("Update JSONB by key query: {}", query);

    sqlx::query(query)
        .bind(key)
        .bind(value)
        .bind(create_if_not_exists)
        .bind(id)

        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_jsonb(table: &str, column: &str, id: i64, value: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("UPDATE {} SET {} = $1 WHERE id = $2", table, column);
    sqlx::query(query)
        .bind(value)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_hstore(table: &str, column: &str, id: i64, value: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let query = &format!("UPDATE {} SET {} = $1::HSTORE WHERE id = $2", table, column);
    sqlx::query(query)
        .bind(value)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// pub async fn empty_map(table: &str, column: &str, id: i64, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
//     let is_hstore = {
//         let query = "SELECT EXISTS (
//             SELECT 1 FROM pg_type WHERE typname = 'hstore'
//         )";

//         let row = sqlx::query(query)
//             .fetch_one(pool)
//             .await?;

//         let exists: bool = row.get(0);
//         exists
//     };
//     if is_hstore {
//         let query = &format!("UPDATE {} SET {} = ''::HSTORE WHERE id = $1", table, column);
//         sqlx::query(query)
//             .bind(id)
//             .execute(pool)
//             .await?;
//         return Ok(());
//     } else {
//     }
//     Ok(())
// }

pub async fn get_table_schema(name: &str, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<JSON, StdError> {
    let query = "SELECT column_name, data_type, is_nullable, column_default
        FROM information_schema.columns
        WHERE table_name = $1";

    let rows = sqlx::query(query)
        .bind(name)
        .fetch_all(pool)
        .await?;

    let mut schema = serde_json::Map::new();
    for row in rows {
        let column_name: String = row.get("column_name");
        let data_type: String = row.get("data_type");
        let is_nullable: String = row.get("is_nullable");
        let column_default: Option<String> = row.get("column_default");

        let mut column_info = serde_json::Map::new();
        column_info.insert("type".to_string(), JSON::String(data_type.clone()));
        column_info.insert("nullable".to_string(), JSON::Bool(is_nullable == "YES"));
        if let Some(default) = column_default {
            column_info.insert("default".to_string(), JSON::String(default));
        } else {
            column_info.insert("default".to_string(), JSON::Null);
        }

        schema.insert(column_name, JSON::Object(column_info));
    }

    Ok(JSON::Object(schema))
}
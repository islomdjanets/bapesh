use sqlx::{pool, postgres::PgPoolOptions, Connection, Postgres, Row};
use crate::json::JSON;

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

pub async fn get_sql_type(r#type: &str, pool: &Pool) -> String {
    let std_type = match r#type {
        "string" => "TEXT",
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

        "Vector2" | "vector2" => "REAL[2]",
        "Vector3" | "vector3" => "REAL[3]",
        "Vector4" | "vector4" => "REAL[4]",
        "Color" | "color" => "SMALLINT[4]", // Assuming Color is a 4-component RGBA color
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
        .fetch_optional(pool)
        .await?;


    if let Some(row) = row {
        let json: JSON = row.try_get("data")?; // Assuming the column is named 'data'
        Ok(Some(json))
    } else {
        Ok(None)
    }
}

pub async fn insert_into_table(name: &str, values: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let (keys, values) = generate_values(values);

    println!("Inserting into table: {}", name);
    println!("Keys: {}", keys);
    println!("Values: {}", values);

    let query = &format!("INSERT INTO {} ({}) VALUES ({})", name, keys, values);

    // Here you would typically bind the values to the query
    // For simplicity, we are not binding any values in this example
    sqlx::query(query)
        .execute(pool)
        .await?;

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

            println!("Processing key: {}, value: {:?}", key, value);
            let value = value.as_str().unwrap_or("NULL");
            // result.push_str(&format!("{}: {}, ", key, value));

            keys.push_str(&format!("{}, ", key));
            values.push_str(&format!("'{}', ", value));

            keys = keys.trim_end_matches(", ").to_string();
            values = values.trim_end_matches(", ").to_string();
        }
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

            if value.is_object() {
                let info = value.as_object().unwrap();
                value = info.get("type").unwrap_or(value);
                let description = info.get("description").unwrap_or(&JSON::Null);
                let default = info.get("default").unwrap_or(&JSON::Null);

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

                let sql_type = get_sql_type(inner_type, pool).await;
                properties.push_str(&format!("{} {}[] DEFAULT '{{}}', ", key, sql_type));
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

                    let sql_type = get_sql_type(inner_type, pool).await;
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
                let sql_type = get_sql_type(value, pool).await;
                let default_value = get_default_value(&sql_type);
                // properties.push_str(&format!("{} {} DEFAULT {} {} {}, ", key, sql_type, default_value, if is_nullable { "" } else { "NOT NULL" }, if is_primary_key { "PRIMARY KEY" } else { "" }));
                properties.push_str(&format!("{} {} DEFAULT {} {}, ", key, sql_type, default_value, if is_primary_key { "PRIMARY KEY" } else { "" }));
            }
        }
    }
    properties = properties.trim_end_matches(", ").to_string();

    return properties;
}

pub fn get_default_value(r#type: &str) -> String {
    println!("Getting default value for type: {}", r#type);
    match r#type {
        "TEXT" => "''",
        "BIGINT" | "INTEGER" | "int" | "u8" | "u16" | "i8" | "i16" | "u32" | "i32" | "u64" | "usize" | "i64" | "isize" => "0",
        "REAL" | "float" | "f32" | "f64" => "0.0",
        "BOOLEAN" | "bool" => "FALSE",

        "TIMESTAMP WITH TIME ZONE" => "CURRENT_TIMESTAMP",
        "TIMESTAMP WITHOUT TIME ZONE" => "CURRENT_TIMESTAMP",
        "DATE" => "CURRENT_DATE",
        "TIME" => "CURRENT_TIME",

        "REAL[2]" => "{0.0, 0.0}",
        "REAL[3]" => "{0.0, 0.0, 0.0}",
        "REAL[4]" => "{0.0, 0.0, 0.0, 0.0}",
        "SMALLINT[4]" => "{0, 0, 0, 0}", // Assuming Color is a 4-component RGBA color

        _ => "NULL", // Default to NULL for unknown types
    }.to_string()
}

pub async fn create_table(name: &str, schema: &JSON, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), StdError> {
    let properties = generate_properties(schema, pool).await;
    println!("Properties: {}", properties);

    let query = &format!("CREATE TABLE {} ({});", name, properties);
    println!("Create table query: {}", query);

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

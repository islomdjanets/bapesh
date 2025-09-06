use std::{collections::HashMap, fs::ReadDir, io::{Read, Write}};
use serde::{Deserialize, Serialize};
use crate::{date, driver::Driver, handshake::Response, json::{self, Array, Object, JSON}};

#[derive(Debug, Serialize, Deserialize)]
enum Log_Type {
    Error,
    Warning,
    Message,
}

#[derive(Debug, Serialize, Deserialize)]
struct Log {
    r#type: Log_Type,
    time: date::Time,
    message: String,
}

impl Log {
    pub fn from_response(response: &Response) -> Self {
        Self {
            message: "".to_string(),
            time: date::now(),
            r#type: Log_Type::Message,
        }
    } 
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Admin {
    name: String,
    password: String,
    email: String,
}

impl Admin {
    fn from_json(json: &JSON) -> Self {
        let admin: Admin = serde_json::from_value(json.clone()).unwrap();

        admin
    } 
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Database_Meta {
    admins: Vec<Admin>,
}

impl Database_Meta {
    fn new() -> Self {
        Self {
            admins: vec![],
        }
    } 

    fn from_json(json: &JSON) -> Self {
        let admins_array = json["admins"].as_array().unwrap();

        let mut admins = vec![];
        admins_array.iter().for_each(|admin_json|{
            admins.push(Admin::from_json(admin_json));
        });

        Self {
            admins
        }
    }
}

pub struct Database {
    name: String,
    collections: HashMap<String, Collection>,
    meta: Database_Meta,
    current_collection: Option<String>,
}

impl Database {
    pub fn new(name: String) -> Self {
        // let file = Driver::open_file(format!("./{name}/meta.json"));
        // let file = ;
        let meta = match Driver::read_json(&format!("./{name}/meta.json")) {
            Some(content) => Database_Meta::from_json(&content),
            None => Database_Meta::new(),
        };

        // println!("{:?}", meta);

        let mut db = Self {
            name: name.clone(),
            collections: HashMap::new(),
            meta,
            current_collection: None
        };
        let dir = Driver::open_directory(format!("./{name}"), true).unwrap();

        db.fill_collections(dir);

        db
    }

    pub fn save(&mut self) {
        self.collections.values_mut().for_each(|collection| {
            collection.save();
        })
    }

    pub fn check_admin(&self) {

    }

    pub fn get_admin(&self, admin_name: &str, admin_password: &str) -> Option<&Admin> {
        self.meta.admins
            .iter()
            .find(|&admin| admin.name == admin_name && admin.password == admin_password)
    }

    pub fn refresh(&mut self) {
        let dir = Driver::open_directory(format!("./{}", self.name), true).unwrap();
        self.fill_collections(dir);
    }

    fn fill_collections(&mut self, dir: ReadDir) {
        dir.for_each(|entry|{
            let dir_entry = entry.unwrap();
            let name = dir_entry.file_name();
            if name != "meta.json" {
                let collection_name = name.to_str().unwrap().to_string();
                self.new_collection(collection_name);
            }
        })
    }

    pub fn update_schema_from_json(&mut self, data: JSON, rewrite: bool) {
        let mut schema = Schema::from_json(data);
        let path = self.get_path_to_collection(&self.current_collection.clone().unwrap());
            let mut schema_file = Driver::open_with_options(
                &format!("{path}/schema.json"),
                true, false, true, true ).unwrap();

            if rewrite {
                schema_file.write_all(schema.to_json().as_bytes()).unwrap();
            }
            // let mut data = String::new();
            // schema_file.read_to_string(&mut data).unwrap();

        self.collections.get_mut(&self.current_collection.clone().unwrap()).unwrap().schema = Some(schema);
    }

    pub fn update_schema(&mut self, collection_name: &str, edited_properties: &Object, new_properties: &Object, deleted_properties: &Array, rewrite: bool) {
        // let collection_name = self.current_collection.unwrap().clone();
        let path = self.get_path_to_collection(collection_name);
        let collection = self.get_collection_mut(collection_name.to_string(), false).unwrap();

        let schema = match collection.schema.as_mut() {
            Some(schema) => schema,
            None => {
                // let mut schema = ;
                &mut Schema::new()
            },
        };
        new_properties.into_iter().for_each(|(name, r#type)|{
            let property = Property::from_json(r#type);
            schema.properties.insert(name.clone(), property);
        });

        edited_properties.into_iter().for_each(|(name, info)|{
            let new_name = &info["name"];
            if !new_name.is_null() { 
                // println!("new name for {}: {}", name, new_name);
                let property = schema.properties.remove(name).unwrap();
                let key = new_name.as_str().unwrap();
                schema.properties.insert(key.to_string(), property);
            }

            let new_type = &info["type"];
            if !new_type.is_null() {
                // println!("new type for {}: {}", name, new_type);
                let property = schema.properties.get_mut(name).unwrap();
                property.r#type = new_type.clone();
            }
        });

        deleted_properties.iter().for_each(|record_id|{
            let key = record_id.as_str().unwrap();
            schema.properties.remove(key);
        });

        if rewrite {
            let schema_path = &format!("{path}/schema.json");
            Driver::erase(schema_path);
            let mut schema_file = Driver::open_with_options(
                schema_path, true, false, true, true).unwrap();

            schema_file.write_all(schema.to_json().as_bytes()).unwrap();
        }

        collection.schema = Some(schema.clone());

        // let mut schema = Schema::from_json(data);
        // let path = self.get_path_to_collection(&self.current_collection.clone().unwrap());
        //     let mut schema_file = Driver::open_with_options(
        //         &format!("{path}/schema.json"),
        //         true, false, true, true ).unwrap();

        //     // let mut data = String::new();
        //     // schema_file.read_to_string(&mut data).unwrap();

        // self.collections.get_mut(&self.current_collection.clone().unwrap()).unwrap().schema = Some(schema);
    }

    // fn get_path(&self) -> String {
    //     format!("./{}", self.name)
    // }
    
    fn get_path_to_collection(&self, collection_name: &str) -> String {
        format!("./{}/{}", self.name, collection_name)
    }

    pub fn has_collection(&mut self, name: &str) -> bool {
        self.collections.contains_key(name)
    }

    pub fn delete_collection(&mut self, name: &str) {
        let result = self.collections.remove(name);
        if result.is_some() {
            let path = self.get_path_to_collection(name); 
            Driver::delete_folder(&path);
        }
    }

    pub fn new_collection(&mut self, name: String) -> &Collection {
        let path = self.get_path_to_collection(&name); 
        // println!("{path}");

        let meta_path = format!("{}/meta.json", path);
        let meta_data = Driver::read_json(&meta_path);
        let meta = match meta_data {
            Some(meta_data) => Meta::from_json(meta_data),
            None => Meta::default(),
        };
        // println!("{:?}", meta);

        let logs_path = format!("{}/logs.json", path);
        let logs_data = Driver::read_json(&logs_path);
        let logs = match logs_data {
            Some(logs_data) => {
                // println!("parse logs from file");
                Vec::new()
            },
            None => Vec::new(),
        };
        let mut collection = Collection::new(path.clone(), meta, logs);

        // let path = format!("./{}/{}", self.name, name.clone());
        let directory = Driver::open_directory(path.clone(), true).unwrap();
        // self.fill_records(directory);
        collection.fill_records(directory);

        self.use_collection(&name);
        self.collections.insert(name.clone(), collection);
        
        // println!("path: {}", path);
        let schema_path = format!("{}/schema.json", path);
        // println!("schema_path: {}", schema_path);

        let schema_source = Driver::read_to_string(&schema_path);
        if let Ok(schema_source) = schema_source {
            // let source = schema_source.unwrap();
            let json = serde_json::from_str(&schema_source);
            if let Ok(json) = json {
                // println!("{}", json);
                self.update_schema_from_json(json, false);
                // println!("schema got from file");
            }
        }

        self.collections.get(&name).unwrap()
        // &collection
    }

    pub fn get_collection(&mut self, name: String, create: bool) -> Option<&Collection> { 
        if self.collections.contains_key(&name) {
            return self.collections.get(&name);
        }
        if create {
            return Some(self.new_collection(name));
        }
        None
    }

    pub fn get_collection_mut(&mut self, name: String, create: bool) -> Option<&mut Collection> { 
        if self.collections.contains_key(&name) {
            return self.collections.get_mut(&name);
        }
        if create {
            self.new_collection(name.clone());
            return self.collections.get_mut(&name);
        }
        None
    }

    pub fn show_dbs() {}

    pub fn get_collections(&mut self) -> &HashMap<String, Collection> {
        // let mut collections = vec![];
        // self.collections.values().for_each(|collection| {
        //     collections.push(collection)
        // });

        &self.collections
    }

    pub fn use_collection(&mut self, collection_name: &str) {
        self.current_collection = Some(collection_name.into());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub r#default: Option<JSON>,
    pub r#type: JSON,
    pub description: Option<JSON>,
    // pub custom: Option<HashMap<String, JSON>>,
}

impl Property {
    pub fn from_json(json: &JSON) -> Self {
        let default = if json["default"].is_null() {
            None
        } else {
            Some(json["default"].clone())
        };

        let mut description = None;
        let description_value = json["description"].clone();
        if !description_value.is_null() {
            description = Some(description_value);
        }

        Self {
            default,
            r#type: json["type"].clone(),
            description,
            // custom: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub parent: Option<String>,
    pub custom: Option<HashMap<String, Property>>,
    pub properties: HashMap<String, Property>,
}

impl Schema {
    pub fn new() -> Self {
        Self {
            parent: None,
            custom: None,
            properties: HashMap::new(),
        }
    }

    pub fn to_json(&mut self) -> String {
        // let mut properties = HashMap::new();

        // if self.custom.is_some() {
        //     properties.insert("custom", self.custom.clone().unwrap());
        // }

        // for property in self.properties.iter() {
        //     properties.insert(property.0, property.1);
        // }

        let parent = self.parent.clone();
        let mut binding = serde_json::to_value(&self.properties).unwrap();
        let properties = binding.as_object_mut().unwrap();

        for property in properties.into_iter() {
            let value = property.1;
            let description = value["description"].clone();
            if description.is_null() {
                value.as_object_mut().unwrap().remove("description");
            }
        }

        if let Some(parent) = parent {
            properties.insert("parent".into(), parent.into());
        }

        // println!("{:?}", properties);

        serde_json::to_string(properties).unwrap()
    }

    pub fn from_json(json: JSON) -> Self {
        let parent = if json["#parent"].is_null() {
            None
        } else {
            Some(json["#parent"].to_string())
        };

        let custom = if json["#custom"].is_null() {
            None
        } else {
            let entries = json["#custom"].as_object().unwrap();

            let mut custom_properties: HashMap<String, Property> = HashMap::new();
            for (key, value) in entries {
                custom_properties.insert(key.to_string(), Property::from_json(value));
            }
            Some(custom_properties)
        };

        let mut properties: HashMap<String, Property> = HashMap::new();

        for (key, value) in json.as_object().unwrap() {
            properties.insert(key.to_string(), Property::from_json(value));
        }

        Self { parent, custom, properties }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

pub trait From_Record<T> {
    fn hydrate( record: &Record ) -> T;
}

#[derive(Debug,Serialize, Deserialize)]
pub struct Record {
    pub created: date::Time,
    pub updated: date::Time,
    pub properties: HashMap<String, JSON>
}

impl Record { // not tested
    pub fn from_file(path: &str) -> Option<Record> {
        let data = Driver::read_to_string(path);
        if let Ok(data) = data {
            let json: Result<JSON, serde_json::Error> = serde_json::from_str(&data);            
            if let Ok(json) = json {

                let properties = json["properties"].clone();
                let created = json["created"].as_i64()?;
                let updated = json["updated"].as_i64()?;
                let mut record = Record {
                    created, // get from file
                    updated, // get from file
                    properties: HashMap::new(),
                };

                let properties = properties.as_object()?;
                properties.iter().for_each(|entry| {
                    let (key, value) = entry;
                    record.properties.insert(key.clone(), value.clone());
                });
                // println!("new record from file generated");
                // println!("{:?}",record);
                return Some(record);
            }

        }

        None
    }

    pub fn from_entry( entry: JSON ) -> Self {
        let mut properties = HashMap::new();
        for property in entry.as_object().unwrap() {
            properties.insert( property.0.clone(), property.1.clone() );
        }

        Self {
            created: 0,
            updated: 0,
            properties,
        }
    } 

    pub fn from_schema(schema: &Schema) -> Self {
        // println!("new record from schema started generating");
        let mut properties = HashMap::new();
        for entry in schema.properties.iter() {
            if entry.0 == "#custom" {
                continue;
            }
            let default = match entry.1.default.clone() {
                Some(default_value) => default_value,
                None => JSON::Null,
            };
            properties.insert(entry.0.clone(), default);
        } 
        // println!("new record from schema generated");

        Self {
            created: date::now(),
            updated: date::now(),
            properties
        }
    }

    pub fn to_json(&self) -> JSON {
        serde_json::json!(self.properties)
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Meta {
    protected_keys: Vec<String>,
}

impl Meta {
    fn new() -> Self {
        Meta {
            protected_keys: vec![],
        }
    }

    fn from_json(json: JSON) -> Meta {
        let protected_keys: Vec<String> = serde_json::from_value(json["protected_keys"].clone()).unwrap();

        Meta {
            protected_keys,
        }
    }
}

#[derive(Debug,Serialize, Deserialize)]
pub struct Collection {
    pub schema: Option<Schema>,
    pub created: date::Time,
    pub updated: date::Time,

    #[serde(skip)]
    meta: Meta,

    #[serde(skip)]
    logs: Vec<Log>,

    #[serde(skip)]
    records: HashMap<String,Record>,
    pub max_records: usize,

    #[serde(skip)]
    path: String,
}

impl Collection {
    pub fn new(path: String, meta: Meta, logs: Vec<Log>) -> Self {
        Self {
            path,
            schema: None,
            created: 0,
            updated: 0,
            meta,
            logs,
            max_records: 0,
            records: HashMap::new(),
        }
    }

    pub fn save(&mut self) {
        // println!("save: {}", self.path);
        let now = date::now();
        self.records.iter().for_each(|(record_name,record)| {
            self.write_record(record, record_name.to_string());
        });

        let mut records = HashMap::new();
        self.records.iter_mut().for_each(|(name, record)| {
            records.insert(name, record.updated);
        });

        for (name, updated) in records.into_iter() {
            let elapsed = now - updated;
            println!("elapsed: {}", elapsed);
            if elapsed > 60 * 3 { 
                // self.remove_record(&name);
                println!("unload: {}", name);
            }
            // if record.updated 
        }
    }

    pub fn is_protected(&self, property: &str) -> bool {
        self.meta.protected_keys.contains(&property.to_string())
    }

    // pub fn get_schema(&mut self, create: bool) -> Schema {
    //     // let schema = self.schema;

    //     if self.schema.is_none() && create == true {
    //         let schema = Schema::new();
    //         self.schema = Some(schema);

    //         return schema.clone();
    //     }

    //     self.schema.unwrap().as.clone()
    // }

    pub fn log(&mut self, message: String) {
        self.logs.push(
            Log {
                message,
                time: date::now(),
                r#type: Log_Type::Message,
            }
        )
    }

    pub fn error(&mut self, message: String) {
        self.logs.push(
            Log {
                message,
                time: date::now(),
                r#type: Log_Type::Error,
            }
        )
    }

    pub fn warn(&mut self, message: String) {
        self.logs.push(
            Log {
                message,
                time: date::now(),
                r#type: Log_Type::Warning,
            }
        )
    }

    pub fn get_records_from_to(&mut self, from: usize, to: usize) {
            // let records = collection.get_records();
            // let mut records = HashMap::new();

            // let collection_path = collection.get_path();
            let dir = Driver::open_directory(self.path.clone(), true).unwrap();

            // let entries = &Vec::from_iter(dir);
            // if !entries.is_empty() {
            //     let mut index = from;

            // } 
            //

        for(index, entry) in dir.enumerate() {
            if let Ok(file) = entry { 
                // index += 1;
                let name = file.file_name().to_str().unwrap().to_string(); // Djami.json
                let mut file_name = name.split('.');
                let name = file_name.next().unwrap();

                if name == "schema" || name == "meta" || name == "logs" {
                    continue;
                }
                // println!("{}", name);
                self.new_record(name.to_string(), false);
                // records.insert(name, self.get_record(name.clone(), false).unwrap());
                // let record = self.get_record_not_mut(name.clone().to_string()).unwrap();
                // println!("{}",record);
                // records.insert(name, record);
            }
        }
    }

    pub fn get_records(&mut self) -> &HashMap<String, Record> {
        // let mut records = vec![];
        //
        // self.records.values().for_each(|record| {
        //     records.push(record);
        // });
        //
        // records

        &self.records
    }

    pub fn get_path(&mut self) -> String {
        self.path.clone()
    }

    fn fill_records(&mut self, dir: ReadDir) -> &mut Self {
        // self.max_records = dir.c.count();

        dir.for_each(|entry|{
            let file_entry = entry.unwrap();
            let name = file_entry.file_name();

            let record_id = name.to_str().unwrap().to_string();
            if record_id != "schema.json" && record_id != "meta.json" && record_id != "logs.json" {
                self.max_records += 1;
            }
            // self.new_collection(record_id);
        });

        self
    }

    pub fn new_record(&mut self, name: String, rewrite: bool) -> Option<&Record> {
        if self.schema.is_none() {
            println!("schema is none");
            return None;
        }

        let schema = self.schema.to_owned()?;
        // println!("schema cloned");

        let path = format!("{}/{name}.json", self.path);

        let record = self.get_new_record(&path, &schema);
        // let mut record_file = Driver::open_with_options(
        //     format!("{}/{name}.json", self.path),
        //     true, false, true, true ).unwrap();
        // record_file.write_all(record.to_string().as_bytes()).unwrap();

        if rewrite {
            self.write_record(&record, name.clone());
        }

        self.records.insert(name.clone(), record);

        self.records.get(&name)
    }

    pub fn new_record_mut(&mut self, name: String, rewrite: bool) -> &mut Self {
        self.new_record(name, rewrite);
        self
    }

    pub fn delete_record(&mut self, name: &str) {
        let result = self.records.remove(name);
        if result.is_some() {
            let path = format!("{}/{name}.json", self.path);
            Driver::erase(&path);
        }
    }

    fn get_new_record(&self, path: &str, schema: &Schema) -> Record {
        let record = Record::from_file(path);
        if let Some(record) = record {
            return record;
        }
        
        Record::from_schema(schema)
    }

    pub fn write_record(&self, record: &Record, name: String) {
        let path = format!("{}/{name}.json", self.path);
        Driver::erase(&path);

        let mut record_file = Driver::open_with_options(
            &path, true, false, true, true ).unwrap();

        record_file.write_all(record.to_string().as_bytes()).unwrap();
    }

    pub fn get_record_not_mut(&self, name: String) -> Option<&Record> {
        // let mut name = name;
        // if name.ends_with(".json") {
        //     let mut parts = name.split('.');
        //     name = parts.next().unwrap().to_string();
        //     // return self.records.get(&name);
        // }
        // self.new_record(name.clone())
        self.records.get(&name)
        // return self.records.get(&name);
    }

    pub fn get_record(&mut self, name: String, create: bool) -> Option<&Record> {
        if self.records.contains_key(&name) {
            return self.records.get(&name);
        }
        if create {
            // println!("new record");
            return self.new_record(name, create);
        }

        None
    }

    pub fn get_record_mut( &mut self, name: String ) -> Option<&mut Record> {
        self.records.get_mut(&name)
    }

    pub fn remove_record(&mut self, name: &str) {
        self.records.remove(name);
    }

    pub fn has_record(&mut self, name: &str) -> bool {
        self.records.contains_key(name)
    }

    pub fn from_json(json: JSON, name: String ) -> Self {
        let mut schema_path: String = json["struct"].as_str().unwrap()[6..].into();
        schema_path.remove(schema_path.len() - 1);

        println!("{}", schema_path);

        let path = schema_path.clone() + ".json";
        let schema_data = Driver::read_json(&path).unwrap();
        let schema = Schema::from_json(schema_data);

        println!("{}",schema_path);
        let meta_path = "";
        let meta_data = Driver::read_json(meta_path).unwrap();
        let meta = Meta::from_json(meta_data);
        println!("{:?}", meta);

        let logs = Vec::new();

        let mut records = HashMap::new();

        let mut index = 0;
        json["entries"].as_array().unwrap().iter().for_each(|entry| {
            // records.push(Record::from_entry(entry.clone()));
            records.insert(index.to_string(), Record::from_entry(entry.clone())); // not tested!!!
            index += 1;
        });

        Self {
            path:schema_path,
            created: 0,
            updated: 0,
            meta,
            logs,
            max_records: 0,
            schema: Some(schema),
            records
        }
    }

    pub fn find() {}

    pub fn insert() {}

    pub fn update() {}

    pub fn remove() {}
}

// impl Default for Collection {
//     fn default() -> Self {
//         Self::new("".into())
//     }
// }

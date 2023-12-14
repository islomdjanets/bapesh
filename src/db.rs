use serde_json;
use std::{collections::HashMap, fmt::format, io::Read};

use crate::driver::Driver;

type JSON = serde_json::Value;

pub struct Database {
    name: String,
    is_realtime: bool,
    collections: HashMap<String, Collection>,
}

impl Database {
    pub fn new( name: String, is_realtime: bool ) -> Self {
        let db = Self {
            name: name.clone(),
            is_realtime,
            collections: HashMap::new()
        };

        let root = Driver::create_directory(format!("./{name}"), true);

        db
    }

    pub fn new_collection( &mut self, name: String, schema: Schema ) -> &Collection {
        let collection = Collection::new( schema );
        self.collections.insert(name.clone(), collection );

        let path = format!("./{}/{}", self.name, name.clone());
        let directory = Driver::create_directory(path.clone(), true);
        let mut schema_file = Driver::open_with_options(
            format!("{path}/schema.json"),
            true, false, true, true ).unwrap();

        let mut data = String::new();
        schema_file.read_to_string(&mut data).unwrap();

        self.collections.get(&name).unwrap()
        // &collection
    }

    pub fn get_collection( &mut self, name: String, create: bool ) -> Option<&Collection> {
        
        if self.collections.contains_key(&name) {
            return self.collections.get(&name);
        }
        if create {
            return Some(self.new_collection(name, Schema::new()));
        }
        None
    }

    pub fn get_collection_mut( &mut self, name: String, create: bool ) -> Option<&mut Collection> {
        
        if self.collections.contains_key(&name) {
            return self.collections.get_mut(&name);
        }
        if create {
            self.new_collection(name.clone(), Schema::new());
            return self.collections.get_mut(&name);
        }
        None
    }

    pub fn show_dbs() {}

    pub fn show_collections() {}

    pub fn use_db(db: String) {}
}

#[derive(Debug)]
pub struct Property {
    pub r#default: Option<JSON>,
    pub r#type: JSON,
    pub description: JSON,
}

impl Property {
    pub fn from_json(json: &JSON) -> Self {
        let default = if json["default"].is_null() {
            None
        } else {
            Some(json["default"].clone())
        };

        Self {
            r#default: default,
            r#type: json["type"].clone(),
            description: json["description"].clone(),
        }
    }
}

#[derive(Debug)]
pub struct Schema {
    pub parent: Option<String>,
    pub properties: HashMap<String, Property>,
}

impl Schema {
    pub fn new() -> Self {
        Self { parent: None, properties: HashMap::new() }
    }

    pub fn from_json(json: JSON) -> Self {
        let parent = if json["#parent"].is_null() {
            None
        } else {
            Some(json["#parent"].to_string())
        };
        let mut properties: HashMap<String, Property> = HashMap::new();

        for (key, value) in json.as_object().unwrap() {
            properties.insert(key.to_string(), Property::from_json(value));
        }

        Self { parent, properties }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Record {
    properties: HashMap<String, Property>
}

impl Record { // not tested
    pub fn from_entry( entry: JSON ) -> Self {

        let mut properties = HashMap::new();
        for property in entry.as_object().unwrap() {
            properties.insert( property.0.clone(), Property::from_json(property.1) );
        }
        Self { properties }
    } 

    pub fn from_schema( schema: &Schema ) -> Self {
        todo!();
    }
}

#[derive(Debug)]
pub struct Collection {
    pub schema: Schema,
    pub records: HashMap<String,Record>,
}


impl Collection {
    pub fn new( schema: Schema ) -> Self {
        Self {
            schema,
            records: HashMap::new(),
        }
    }

    pub fn new_record( &mut self, name: String ) -> Option<&Record> {
        self.records.insert(name.clone(), Record::from_schema(&self.schema));
        self.records.get(&name)
    }

    pub fn get_record( &mut self, name: String, create: bool ) -> Option<&Record> {
        if self.records.contains_key(&name) {
            return self.records.get(&name);
        }
        if create {
            return self.new_record(name);
        }

        None
    }

    pub fn get_record_mut( &mut self, name: String ) -> Option<&mut Record> {
        self.records.get_mut(&name)
    }

    pub fn from_json(json: JSON, name: String ) -> Self {
        let mut schema_path: String = json["struct"].as_str().unwrap()[6..].into();
        schema_path.remove(schema_path.len() - 1);

        println!("{}", schema_path);

        let schema_data = Driver::read_json(schema_path + ".json").unwrap();
        let schema = Schema::from_json(schema_data);

        let mut records = HashMap::new();

        let mut index = 0;
        json["entries"].as_array().unwrap().iter().for_each(|entry| {
            // records.push(Record::from_entry(entry.clone()));
            records.insert(index.to_string(), Record::from_entry(entry.clone())); // not tested!!!
            index += 1;
        });

        Self { schema, records }
    }

    pub fn find() {}

    pub fn insert() {}

    pub fn update() {}

    pub fn remove() {}
}


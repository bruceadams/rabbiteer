use amqp::protocol::basic::{BasicProperties, Deliver};
use amqp::{Table, TableEntry};
use error::RbtError;
use rustc_serialize::base64::{self, ToBase64};
use rustc_serialize::json::{self, Json, Object};

#[derive(RustcEncodable)]
struct MsgDeliver {
    consumer_tag: String,
    delivery_tag: u64,
    redelivered: bool,
    exchange: String,
    routing_key: String,
}

#[derive(RustcEncodable)]
struct MsgProps {
    content_type: String,
    headers: Object,
}

#[derive(RustcEncodable)]
struct Msg {
    deliver: MsgDeliver,
    props: MsgProps,
    data: Json,
}

pub fn build_output(
    info: bool,
    deliver: &Deliver,
    props: &BasicProperties,
    body: Vec<u8>,
) -> Result<Vec<u8>, RbtError> {
    if info {
        // delivery info
        let mdel = MsgDeliver {
            consumer_tag: deliver.consumer_tag.clone(),
            delivery_tag: deliver.delivery_tag.clone(),
            redelivered: deliver.redelivered.clone(),
            exchange: deliver.exchange.clone(),
            routing_key: deliver.routing_key.clone(),
        };

        let content_type = props.content_type.clone().unwrap_or(String::from(""));

        // properties
        let mut mprops = MsgProps {
            content_type: content_type.clone(),
            headers: Object::new(),
        };

        if let Some(ref table) = props.headers {
            mprops.headers = table_to_json(table);
        }

        // the body
        let data = figure_out_body(content_type, body)?;

        // and put it together
        let msg = Msg {
            deliver: mdel,
            props: mprops,
            data: data,
        };

        // encode
        let js = json::as_pretty_json(&msg);

        // convert to bytes
        Ok(js.to_string().as_bytes().to_owned())
    } else {
        let content_type = props.content_type.clone().unwrap_or(String::from(""));

        match content_type.as_ref() {
            "application/json" => {
                // interpret body so we can pretty print it
                let body = figure_out_body(content_type, body)?;

                // encode back as pretty
                let js = json::as_pretty_json(&body);

                // convert to bytes
                Ok(js.to_string().as_bytes().to_owned())
            }

            // just return untranslated bytes
            _ => Ok(body),
        }
    }
}

fn figure_out_body(content_type: String, body: Vec<u8>) -> Result<Json, RbtError> {
    // depending on content type, do something
    match content_type.as_ref() {
        "application/json" => Ok(Json::from_str(&String::from_utf8(body)?)?),
        _ => Ok(match content_type.find("text/") {
            Some(_) => Json::String(String::from_utf8(body)?),
            _ => Json::String(body.to_base64(base64::STANDARD)),
        }),
    }
}

fn table_to_json(table: &Table) -> Object {
    let mut ret = Object::new();
    for (skey, entry) in table {
        ret.insert(skey.clone().to_string(), entry_to_json(&entry));
    }
    ret
}

fn entry_to_json(entry: &TableEntry) -> Json {
    match *entry {
        TableEntry::Bool(v) => Json::Boolean(v),
        TableEntry::ShortShortInt(v) => Json::I64(v as i64),
        TableEntry::ShortShortUint(v) => Json::U64(v as u64),
        TableEntry::ShortInt(v) => Json::I64(v as i64),
        TableEntry::ShortUint(v) => Json::U64(v as u64),
        TableEntry::LongInt(v) => Json::I64(v as i64),
        TableEntry::LongUint(v) => Json::U64(v as u64),
        TableEntry::LongLongInt(v) => Json::I64(v),
        TableEntry::LongLongUint(v) => Json::U64(v),
        TableEntry::Float(v) => Json::F64(v as f64),
        TableEntry::Double(v) => Json::F64(v),
        TableEntry::LongString(ref v) => Json::String(v.clone()),
        TableEntry::Void => Json::Null,
        TableEntry::FieldTable(ref v) => Json::Object(table_to_json(&v)),
        TableEntry::Timestamp(v) => Json::U64(v as u64), // maybe string date?
        TableEntry::FieldArray(ref vs) => {
            let mut ret: Vec<Json> = Vec::new();
            for v in vs {
                ret.push(entry_to_json(v))
            }
            Json::Array(ret)
        }
        TableEntry::DecimalValue(decimals, v) => {
            let ten: f64 = (10 as u64).pow(decimals as u32) as f64;
            let dec: f64 = (v as f64) / ten;
            Json::F64(dec)
        }
        //_                             => Err(format!("Cant translate {:?}", entry)),
        // TableEntry::ShortString(ref v) => Ok(Json::String(v.clone())),
    }
}

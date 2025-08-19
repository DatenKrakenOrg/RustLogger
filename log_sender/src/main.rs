use dotenv::dotenv;
use reqwest;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
#[tokio::main]
async fn main() {
    dotenv().ok();
    let endless: bool = env::var("ENDLESS")
        .unwrap()
        .parse()
        .expect("Failed to load endless");
    let repetitions: i32 = env::var("REPETITIONS")
        .unwrap()
        .parse()
        .expect("Failed to load repetitions");
    if endless {
        loop {
            process_file().await;
        }
    } else {
        for _n in 0..repetitions {
            process_file().await;
        }
    }
}

async fn process_file() {
    let client = reqwest::Client::new();
    // File hosts.txt must exist in the current path
    let lines = read_lines(env::var("LOGFILE_PATH").unwrap()).unwrap();
    // Consumes the iterator, returns an (Optional) Strin
    for line in lines.map_while(Result::ok) {
        send_value(&client, line).await
    }
}
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
async fn send_value(client: &reqwest::Client, line: String) {
    let mut data = line.split(",");

    let mut request_body = HashMap::new();
    request_body.insert("timestamp", data.next().unwrap());
    request_body.insert("level", data.next().unwrap());
    request_body.insert("humidity", data.next().unwrap());
    request_body.insert("temperature", data.next().unwrap());
    request_body.insert("", data.next().unwrap());
    let message_parts: Vec<&str> = data.collect();
    let message = get_message(&message_parts);
    request_body.insert("msg", message.as_str());

    let res = client
        .post(env::var("ENDPOINT").unwrap())
        .json(&request_body)
        .send()
        .await;
    match res {
        Ok(response) => println!("sending suceeeded with code {}", response.status()),
        Err(error) => println!("{}", error.to_string()),
    }
}

fn get_message(data_collection: &[&str]) -> String {
    let mut data = data_collection.into_iter();
    let mut message = String::from("");
    let mut option = data.next();
    while !option.is_none() {
        message.push_str(option.unwrap());
        message.push(',');
        option = data.next();
    }
    message.pop();
    message.pop();
    let mut chars = message.chars();
    chars.next();
    message = chars.as_str().to_owned();
    message = message.replace("\"\"", "\"");
    //println!("{}",message);
    message
}

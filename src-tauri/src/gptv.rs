use reqwest::Client;
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::env;

#[derive(Deserialize)]
struct Message {
    content: String,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Response {
    choices: Vec<Choice>,
}


pub async fn send_request(msg: String, base64_png: String) -> Result<String, String> {
    let prompt = "You are a helpful assistant named 'Snippy' in the style of 'Clippy' from Microsoft Office. \
You can see the current window that the user is looking at. You can answer questions the user has about the current window. \
If the user seems to have no specific question, feel free to offer advice on what they are currently looking at, but try to be concise. \
";
    let body = json!({
        "model": "gpt-4-turbo",
        "messages": [
            {
                "role": "system",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    }
                ]
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": msg
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", base64_png)
                        }
                    }
                ]
            }
        ],
        "max_tokens": 300
    });

    let client = Client::new();
    let url = "https://api.openai.com/v1/chat/completions";
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(value) => value,
        Err(e) => return Err(e.to_string())
    };
    let response = client.post(url)
                        .bearer_auth(api_key)
                        .json(&body)
                        .send()
                        .await.unwrap();
    if response.status().is_success() {
        let response = response.json::<Response>().await.unwrap();
        if let Some(first_choice) = response.choices.get(0) {
            println!("Response: {}", first_choice.message.content);
            return Ok(first_choice.message.content.to_string());
        }
        
    } else {
        let error_message = response.text().await.unwrap();
        println!("Failed to get a response: {}", error_message);
    };

    Err("Failed to get a response".to_string())
}
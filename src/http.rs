use anyhow::{Context, Result, anyhow};
use reqwest::blocking::{Client, Response};
use reqwest::Method;
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct ResponseData {
    pub status: u16,
    pub headers: Map<String, Value>,
    pub body: Value,
}

pub struct HttpClient {
    client: Client,
    api_key: String,
}

impl HttpClient {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::builder()
            .user_agent("plane-cli")
            .build()
            .context("build http client")?;
        Ok(Self { client, api_key })
    }

    pub fn execute(
        &self,
        method: &str,
        url: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<ResponseData> {
        let method = Method::from_bytes(method.as_bytes()).context("invalid http method")?;
        let mut req = self
            .client
            .request(method, url)
            .header("x-api-key", &self.api_key)
            .header("accept", "application/json")
            .query(query);

        if let Some(value) = body {
            req = req.header("content-type", "application/json").json(&value);
        }

        let resp = req.send().context("send request")?;
        parse_response(resp)
    }
}

fn parse_response(resp: Response) -> Result<ResponseData> {
    let status = resp.status().as_u16();
    let mut headers = Map::new();
    for (key, value) in resp.headers().iter() {
        if let Ok(val) = value.to_str() {
            headers.insert(key.as_str().to_string(), Value::String(val.to_string()));
        }
    }

    let text = resp.text().context("read response body")?;
    let body = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(_) => Value::String(text),
    };

    Ok(ResponseData {
        status,
        headers,
        body,
    })
}

pub fn ensure_success(status: u16, body: &Value) -> Result<()> {
    if (200..300).contains(&status) {
        return Ok(());
    }
    Err(anyhow!("http {}: {}", status, body))
}

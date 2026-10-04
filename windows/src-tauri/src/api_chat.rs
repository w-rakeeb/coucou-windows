// Direct API chat. Credentials and file contents remain on the native side.
use serde_json::{json,Value};
use tokio::sync::Mutex;
use crate::{claude::{ChatContext,ChatReply},secrets};

#[derive(Default)]
pub struct ApiChat { history: Mutex<History> }
#[derive(Default)]
struct History { binding:String, messages:Vec<Value> }
const PROMPT:&str="You are Mochi, a personal assistant at the top of the user's screen. Answer in the user's language. Use plain text and line breaks. Be clear and helpful. Do not claim to have searched the web or run tools.";

impl ApiChat {
    pub async fn reset(&self){*self.history.lock().await=History::default();}
}

pub async fn send(chat:&ApiChat,provider:&str,model:&str,query:String,context:Option<ChatContext>)->Result<ChatReply,String>{
    let(key_name,label,endpoint)=match provider{
        "openai"=>("openai-api-key","OpenAI","https://api.openai.com/v1/responses"),
        "openrouter"=>("openrouter-api-key","OpenRouter","https://openrouter.ai/api/v1/chat/completions"),
        _=>return Err("Unknown chat provider. Select one in Settings.".into())
    };
    let key=secrets::get(key_name).ok_or_else(||format!("{label} key missing. Open Settings → Chat API."))?;
    let model=model.trim();if model.is_empty(){return Err("Enter a model ID in Settings.".into())}
    let mut history=chat.history.lock().await;
    let binding=format!("{provider}:{model}");
    if history.binding!=binding{history.binding=binding;history.messages.clear();}
    let mut parts=vec![];
    if history.messages.is_empty(){
        if let Some(context)=context{
            match context{
                ChatContext::File{name,path}=>{
                    let size=std::fs::metadata(&path).map_err(|_|"The attached file could not be read.")?.len();
                    if size>25_000_000{return Err("The attached file is too large (maximum 25 MB).".into())}
                    let block=crate::claude::file_block(&path).ok_or("This file cannot be attached. Use an image, PDF, or a text/code file under 200 KB.")?;
                    parts.push(compatible_file(&name,block)?);
                    parts.push(json!({"type":"text","text":format!("File: {name}")}));
                }
                ChatContext::Window{app_name,title,url}=>parts.push(json!({"type":"text","text":format!("App: {app_name}, Window: {title}, URL: {}",url.unwrap_or_default())})),
            }
        }
    }
    parts.push(json!({"type":"text","text":query}));
    // Commit history only after a successful reply. Failed turns do not leak
    // into retries, and the async lock serializes overlapping sends/resets.
    let user=json!({"role":"user","content":parts});
    let mut messages=history.messages.clone();messages.push(user.clone());
    let body=request_body(provider,model,&messages);
    let response=call(endpoint,label,&key,&body).await?;
    let text=reply_text(provider,&response)?;
    history.messages.push(user);history.messages.push(json!({"role":"assistant","content":text}));
    Ok(ChatReply{text})
}

fn compatible_file(name:&str,block:Value)->Result<Value,String>{
    if block["type"]=="text"{return Ok(block)}
    let source=&block["source"];
    let media=source["media_type"].as_str().ok_or("Unsupported file type")?;
    let data=source["data"].as_str().ok_or("Cannot encode file")?;
    let url=format!("data:{media};base64,{data}");
    if block["type"]=="image"{Ok(json!({"type":"image_url","image_url":{"url":url}}))}
    else{Ok(json!({"type":"file","file":{"filename":name,"file_data":url}}))}
}

fn response_message(message:&Value)->Value{
    let Some(parts)=message["content"].as_array() else{return message.clone()};
    let parts:Vec<Value>=parts.iter().map(|p|match p["type"].as_str(){
        Some("image_url")=>json!({"type":"input_image","image_url":p["image_url"]["url"]}),
        Some("file")=>json!({"type":"input_file","filename":p["file"]["filename"],"file_data":p["file"]["file_data"]}),
        _=>json!({"type":"input_text","text":p["text"]})
    }).collect();
    json!({"role":message["role"],"content":parts})
}

fn request_body(provider:&str,model:&str,messages:&[Value])->Value{
    if provider=="openai"{
        json!({"model":model,"instructions":PROMPT,"input":messages.iter().map(response_message).collect::<Vec<_>>(),"max_output_tokens":4096,"store":false})
    }else{
        let mut input=vec![json!({"role":"system","content":PROMPT})];input.extend_from_slice(messages);
        json!({"model":model,"messages":input,"max_tokens":4096,"stream":false})
    }
}

fn reply_text(provider:&str,response:&Value)->Result<String,String>{
    let text=if provider=="openai"{
        response["output"].as_array().into_iter().flatten()
            .filter(|m|m["type"]=="message")
            .flat_map(|m|m["content"].as_array().into_iter().flatten())
            .filter(|p|p["type"]=="output_text")
            .filter_map(|p|p["text"].as_str()).collect::<Vec<_>>().join("\n")
    }else{
        let content=&response["choices"][0]["message"]["content"];
        content.as_str().map(str::to_owned).unwrap_or_else(||content.as_array().into_iter().flatten().filter_map(|p|p["text"].as_str()).collect::<Vec<_>>().join("\n"))
    };
    if text.trim().is_empty(){return Err("The provider returned no text. Check the model's capabilities or try again.".into())}
    Ok(text.trim().to_owned())
}

async fn call(endpoint:&str,label:&str,key:&str,body:&Value)->Result<Value,String>{
    let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(90)).build().map_err(|_|"Could not create API client")?;
    let response=client.post(endpoint).bearer_auth(key).json(body).send().await.map_err(|_|format!("Cannot reach {label}. Check your connection and try again."))?;
    let status=response.status();let value:Value=response.json().await.map_err(|_|format!("{label} returned an invalid response."))?;
    if !status.is_success()||value.get("error").is_some(){
        let detail=value["error"]["message"].as_str().unwrap_or("Request failed. Check the API key, model and account balance.").replace(key,"[redacted]");
        return Err(format!("{label} API {}: {}",status.as_u16(),detail.chars().take(300).collect::<String>()));
    }
    Ok(value)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn formats_images_files_and_history_for_each_provider(){
        let image=compatible_file("a.png",json!({"type":"image","source":{"media_type":"image/png","data":"aW1hZ2U="}})).unwrap();
        let pdf=compatible_file("a.pdf",json!({"type":"document","source":{"media_type":"application/pdf","data":"cGRm"}})).unwrap();
        let history=vec![json!({"role":"user","content":[image,pdf,{"type":"text","text":"Read these"}]}),json!({"role":"assistant","content":"Done"}),json!({"role":"user","content":[{"type":"text","text":"Continue"}]})];
        let openai=request_body("openai","gpt-4.1-mini",&history);
        assert_eq!(openai["store"],false);assert_eq!(openai["input"][0]["content"][0]["type"],"input_image");
        assert_eq!(openai["input"][0]["content"][1]["type"],"input_file");assert_eq!(openai["input"][1]["content"],"Done");
        let router=request_body("openrouter","openai/gpt-4.1-mini",&history);
        assert_eq!(router["messages"][1]["content"][0]["type"],"image_url");assert_eq!(router["messages"][1]["content"][1]["type"],"file");
        assert_eq!(router["messages"].as_array().unwrap().len(),4);
    }
    #[test]
    fn extracts_text_after_reasoning_and_rejects_empty_results(){
        assert_eq!(reply_text("openai",&json!({"output":[{"type":"reasoning"},{"type":"message","content":[{"type":"output_text","text":"Hello"},{"type":"output_text","text":"world"}]}]})).unwrap(),"Hello\nworld");
        assert_eq!(reply_text("openrouter",&json!({"choices":[{"message":{"content":"Hello"}}]})).unwrap(),"Hello");
        assert!(reply_text("openai",&json!({"output":[]})).is_err());
    }
    #[test]
    fn api_transport_uses_bearer_auth_and_handles_provider_errors(){
        use std::{io::{Read,Write},net::TcpListener};
        for status in [200,401]{
            let server=TcpListener::bind("127.0.0.1:0").unwrap();let address=server.local_addr().unwrap();
            let worker=std::thread::spawn(move||{
                let(mut stream,_)=server.accept().unwrap();stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let mut bytes=vec![];let mut part=[0;2048];
                loop{let n=stream.read(&mut part).unwrap();bytes.extend_from_slice(&part[..n]);
                    if let Some(end)=bytes.windows(4).position(|w|w==b"\r\n\r\n"){
                        let headers=String::from_utf8_lossy(&bytes[..end]).to_lowercase();let length=headers.lines().find_map(|l|l.strip_prefix("content-length: ")).unwrap().parse::<usize>().unwrap();
                        if bytes.len()>=end+4+length{assert!(headers.contains("authorization: bearer fixture-key"));assert!(String::from_utf8_lossy(&bytes[end+4..]).contains("fixture-model"));break}
                    }
                }
                let payload=if status==200{r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"ok"}]}]}"#}else{r#"{"error":{"message":"invalid fixture-key"}}"#};
                write!(stream,"HTTP/1.1 {status} test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",payload.len()).unwrap();
            });
            let rt=tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            let result=rt.block_on(call(&format!("http://{address}"),"OpenAI","fixture-key",&request_body("openai","fixture-model",&[])));
            if status==200{assert_eq!(reply_text("openai",&result.unwrap()).unwrap(),"ok")}else{let e=result.unwrap_err();assert!(e.contains("401"));assert!(!e.contains("fixture-key"))}
            worker.join().unwrap();
        }
    }
}

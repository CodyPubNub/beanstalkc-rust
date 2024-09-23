use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::command::Status;
use crate::error::{BeanstalkcError, BeanstalkcResult};
use crate::response::Response;

#[derive(Debug)]
pub struct Request<'b> {
    stream: &'b mut BufReader<TcpStream>,
}

impl<'b> Request<'b> {
    pub fn new(stream: &'b mut BufReader<TcpStream>) -> Self {
        Request { stream }
    }

    pub async fn send(&mut self, message: &[u8]) -> BeanstalkcResult<Response> {
        let _ = self.stream.write(message).await?;
        self.stream.flush().await?;

        let mut line = String::new();
        self.stream.read_line(&mut line).await?;

        if line.trim().is_empty() {
            return Err(BeanstalkcError::UnexpectedResponse(
                "empty response".to_string(),
            ));
        }

        let line_parts: Vec<_> = line.split_whitespace().collect();

        let mut response = Response {
            status: Status::from_str(line_parts.first().unwrap_or(&""))?,
            params: line_parts[1..].iter().map(|&x| x.to_string()).collect(),
            ..Default::default()
        };

        let body_byte_count = match response.status {
            Status::Ok => response.get_int_param(0)?,
            Status::Reserved => response.get_int_param(1)?,
            Status::Found => response.get_int_param(1)?,
            _ => {
                return Ok(response);
            }
        } as usize;

        let mut tmp: Vec<u8> = vec![0; body_byte_count + 2]; // +2 trailing line break
        let body = &mut tmp[..];
        self.stream.read_exact(body).await?;
        tmp.truncate(body_byte_count);
        response.body = Some(tmp);

        Ok(response)
    }
}

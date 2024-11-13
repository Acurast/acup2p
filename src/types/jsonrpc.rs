#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Message {
  Request(Request),
  Response(Response),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Version {
  #[serde(rename = "2.0")]
  V2,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
  pub jsonrpc: Version,
  pub id: String,
  pub method: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub params: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Response {
  Success(SuccessResponse),
  Error(ErrorResponse),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuccessResponse {
  pub jsonrpc: Version,
  pub id: String,
  pub result: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
  pub jsonrpc: Version,
  pub id: String,
  pub error: Error,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Error {
  code: i32,
  message: String,

  #[serde(skip_serializing_if = "Option::is_none")]
  data: Option<serde_json::Value>,
}
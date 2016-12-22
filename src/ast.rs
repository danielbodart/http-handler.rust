#[derive(PartialEq, Debug)]
pub struct HttpVersion {
    pub major: u8,
    pub minor: u8,
}

#[derive(PartialEq, Debug)]
pub struct RequestLine<'a> {
    pub method: &'a str,
    pub request_target: &'a str,
    pub version: HttpVersion,
}

#[derive(PartialEq, Debug)]
pub struct StatusLine<'a> {
    pub version: HttpVersion,
    pub code: u8,
    pub description: &'a str,
}

#[derive(PartialEq, Debug)]
pub enum StartLine<'a> {
    RequestLine (RequestLine<'a>),
    StatusLine (StatusLine<'a>),
}

#[derive(PartialEq, Debug)]
pub struct HttpMessage<'a> {
    pub start_line: StartLine<'a>,
    pub headers: Headers<'a>,
    pub body: MessageBody<'a>,
}

#[derive(PartialEq, Debug)]
pub enum MessageBody<'a> {
    None,
    Slice(&'a [u8]),
}

#[derive(PartialEq, Debug)]
pub struct Headers<'a> (pub Vec<(&'a str, String)>);

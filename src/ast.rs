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
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
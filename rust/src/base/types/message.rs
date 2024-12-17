use std::fmt;

macro_rules! protocol_message {
    ($name:ident) => {
        #[cfg_attr(
            any(target_os = "android", target_os = "ios"),
            derive(uniffi::Record, Debug, Clone)
        )]
        #[cfg_attr(
            not(any(target_os = "android", target_os = "ios")),
            derive(Debug, Clone)
        )]
        pub struct $name {
            pub protocol: String,
            pub bytes: Vec<u8>,
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "{}(protocol={}, bytes={:02x?})",
                    stringify!($name),
                    self.protocol,
                    self.bytes
                )
            }
        }
    };
    ($name:ident { id: $id_type:ty }) => {
        #[cfg_attr(
            any(target_os = "android", target_os = "ios"),
            derive(uniffi::Record, Debug, Clone)
        )]
        #[cfg_attr(
            not(any(target_os = "android", target_os = "ios")),
            derive(Debug, Clone)
        )]
        pub struct $name {
            pub protocol: String,
            pub bytes: Vec<u8>,
            pub(crate) id: $id_type,
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "{}(protocol={}, id={}, bytes={:02x?})",
                    stringify!($name),
                    self.protocol,
                    self.id,
                    self.bytes
                )
            }
        }
    };
}

pub type InboundRequestId = String;
pub type InboundResponseId = String;
pub type OutboundRequestId = String;
pub type OutboundResponseId = InboundRequestId;

protocol_message!(InboundRequest {
    id: InboundRequestId
});
protocol_message!(InboundResponse {
    id: InboundResponseId
});
protocol_message!(OutboundRequest);
protocol_message!(OutboundResponse {
    id: OutboundResponseId
});

#[cfg_attr(
    any(target_os = "android", target_os = "ios"),
    derive(uniffi::Enum, Debug, Clone)
)]
#[cfg_attr(
    not(any(target_os = "android", target_os = "ios")),
    derive(Debug, Clone)
)]
pub enum OutboundMessage {
    Request(OutboundRequest),
    Response(OutboundResponse),
}

impl OutboundMessage {
    pub fn new_request(protocol: String, bytes: Vec<u8>) -> Self {
        Self::Request(OutboundRequest { protocol, bytes })
    }

    pub fn new_response(request: InboundRequest, bytes: Vec<u8>) -> Self {
        Self::Response(OutboundResponse {
            protocol: request.protocol,
            bytes,
            id: request.id,
        })
    }
}

impl fmt::Display for OutboundMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutboundMessage::Request(message) => write!(f, "Request({message})"),
            OutboundMessage::Response(message) => write!(f, "Response({message})"),
        }
    }
}

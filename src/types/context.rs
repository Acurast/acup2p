pub trait Context: Clone {
    type InboundRequest: std::fmt::Debug + Clone;
    type InboundResponse: std::fmt::Debug + Clone;
}

#[derive(Debug, Clone)]
pub struct Empty();

#[derive(Debug, Clone)]
pub enum OutboundMessageContext<C>
where
    C: Context,
{
    Empty,
    InboundRequest(C::InboundRequest),
}

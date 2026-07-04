use pnet::transport::{transport_channel, TransportChannelType};
use pnet::packet::ip::IpNextHeaderProtocols;
fn main() {
    println!("{:?}", TransportChannelType::Layer3(IpNextHeaderProtocols::Tcp));
}

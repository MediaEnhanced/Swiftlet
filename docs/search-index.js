var searchIndex = JSON.parse('{\
"swiftlet_quic":{"doc":"Providing real-time internet communications using the QUIC …","t":"IDLLLKLKALLKLLKLLLNNNDNNNEGNNNNNNNNNDEENNNNNNNNNNNNNNNNNENNNNNNNNNNNNNLMLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLMMMLLLLLLLLLMMLLLLLLLLMLLLLLLLLLLLLLLLLLLLLLLLML","n":["EndpointEventCallbacks","EndpointHandler","background_stream_recv","borrow","borrow_mut","connection_ended","connection_ending_warning","connection_started","endpoint","from","into","main_stream_recv","new","run_event_loop","tick","try_from","try_into","type_id","AeadLimitReached","ApplicationError","BackgroundStreamFinished","Config","ConfigCreation","ConnectionClose","ConnectionCreation","ConnectionEndReason","ConnectionId","ConnectionIdLimitError","ConnectionNotFound","ConnectionPing","ConnectionRecv","ConnectionRefused","ConnectionSend","CryptoBufferExceeded","CryptoErrorEnd","CryptoErrorStart","Endpoint","EndpointCloseReason","Error","FinalStateError","FlowControlError","FrameEncodingError","IdleTimeout","InternalError","InvalidToken","IsServer","KeyUpdateError","LocalApplication","LocalEndpoint","MainStreamFinished","NoError","NoViablePath","PeerApplication","PeerEndpoint","ProtocolViolation","Randomness","SocketAddr","SocketCreation","SocketRecv","SocketSend","StreamCreation","StreamLimitError","StreamRecv","StreamSend","StreamStateError","TransportParameterError","Uncertain","UnexpectedClose","V4","V6","add_client_connection","background_recv_first_bytes","background_stream_send","borrow","borrow","borrow","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","borrow_mut","borrow_mut","borrow_mut","clone","clone_into","close_connection","cmp","eq","fmt","fmt","fmt","fmt","fmt","from","from","from","from","from","from","from","from","from","from_str","get_connection_socket_addr","get_num_connections","hash","idle_timeout_in_ms","initial_background_recv_size","initial_main_recv_size","into","into","into","into","into","into","ip","is_ipv4","is_ipv6","keep_alive_timeout","main_recv_first_bytes","main_stream_send","new","new_client","new_client_with_first_connection","new_server","parse_ascii","partial_cmp","port","reliable_stream_buffer","set_ip","set_port","to_owned","to_socket_addrs","to_string","try_from","try_from","try_from","try_from","try_from","try_from","try_into","try_into","try_into","try_into","try_into","try_into","type_id","type_id","type_id","type_id","type_id","type_id","unreliable_stream_buffer","update_keep_alive_duration"],"q":[[0,"swiftlet_quic"],[18,"swiftlet_quic::endpoint"],[156,"core::option"],[157,"core::time"],[158,"core::result"],[159,"core::any"],[160,"alloc::vec"],[161,"core::cmp"],[162,"core::fmt"],[163,"core::fmt"],[164,"core::net::socket_addr"],[165,"core::convert"],[166,"core::net::parser"],[167,"core::hash"],[168,"core::option"],[169,"alloc::string"]],"d":["Required QUIC Endpoint Handler Event Callback Functions","Main library structure that handles the QUIC Endpoint","Called when there is something to read on the background …","","","Called when a connection has ended and should be cleaned …","Called when a connection is in the process of ending and …","Called when a new connection is started and is application …","QUIC Endpoint Module","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","Called when there is something to read on the main stream.","Create a QUIC Endpoint Handler by giving it an already …","QUIC Endpoint Handler Event Loop","Called when the next tick occurrs based on the tick …","","","","Excessive use of packet protection keys","Application error","Background stream finished","The Endpoint Configuration Structure","Error with the Quic Config Creation","Error closing a connection","Error creating a connection","Reason the connection has ended / is ending","A Connection ID used to communicate with the endpoint …","Too many connection IDs received","Cannot find connection from Connection ID","Error sending out a PING","Error having a connection process the received data","Server refuses a connection","Error getting send data from a connection","CRYPTO data buffer overflowed","TLS Alert End","TLS Alert Start","The Quic Endpoint structure","Based on combination of QUIC Transport Error Codes and …","Errors that the QUIC Endpoint can return","Change to final size","Flow control error","Frame encoding error","Idle Timeout","Implementation Error","Invalid Token received","Error trying to perform a client Endpoint operation on a …","Invalid packet protection update","Local Application Error","Local Endpoint Error","Main stream finished","No Error","No viable network path exists","Peer Application Error","Peer Endpoint Error","Generic protocol violation","Error with creating or using the randomness structure / …","An internet socket address, either IPv4 or IPv6.","Error with the UDP socket creation","Error receiving data on the UDP socket","Error sending data on the UDP socket","Error finishing the connection establishment process and …","Too many streams opened","Error receiving data from the stream","Error sending data on the stream","Frame received in invalid stream state","Error in transport parameters","Not sure of the reason","Error from an unexpected close","An IPv4 socket address.","An IPv6 socket address.","Add a connection for a Client Endpoint","The number of bytes to receive on the background stream …","Send data over the background stream","","","","","","","","","","","","","","","Close a connection with a given error code value","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Converts a <code>SocketAddrV4</code> into a <code>SocketAddr::V4</code>.","Converts a <code>SocketAddrV6</code> into a <code>SocketAddr::V6</code>.","Converts a tuple struct (Into&lt;<code>IpAddr</code>&gt;, <code>u16</code>) into a …","Returns the argument unchanged.","","Get the socket address for a connection","Get the number of connections that the Endpoint is managing","","The quic connection idle timeout in milliseconds.","The initial background stream recieve buffer size.","The initial main stream recieve buffer size.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Returns the IP address associated with this socket address.","Returns <code>true</code> if the IP address in this <code>SocketAddr</code> is an …","Returns <code>true</code> if the IP address in this <code>SocketAddr</code> is an …","The keep alive timeout duration.","The number of bytes to receive on the main stream before …","Send data over the main stream","Creates a new socket address from an IP address and a port …","Create a QUIC Client Endpoint","Create a QUIC Client Endpoint with an initial connection","Create a QUIC Server Endpoint","Parse a socket address from a slice of bytes.","","Returns the port number associated with this socket …","The quic connection bidirectional stream receive buffer …","Changes the IP address associated with this socket address.","Changes the port number associated with this socket …","","","","","","","","","","","","","","","","","","","","","","The quic connection unidirectional stream receive buffer …","Update the keep alive duration time"],"i":[0,0,10,11,11,10,10,10,0,11,11,10,11,11,10,11,11,11,23,23,23,0,13,13,13,0,0,23,13,13,13,23,13,23,23,23,0,0,0,23,23,23,7,23,23,13,23,7,7,23,23,23,7,7,23,13,0,13,13,13,13,23,13,13,23,23,7,13,16,16,1,32,1,32,1,13,23,7,16,32,1,13,23,7,16,16,16,1,16,16,13,23,7,16,16,32,1,13,23,7,16,16,16,16,16,1,1,16,32,32,32,32,1,13,23,7,16,16,16,16,32,32,1,16,1,1,1,16,16,16,32,16,16,16,16,16,32,1,13,23,7,16,32,1,13,23,7,16,32,1,13,23,7,16,32,1],"f":[0,0,[[-1,1,2,[4,[3]]],[[6,[5]]],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[-1,1,2,7,5],8,[]],[[-1,1,2,7],9,[]],[[-1,1,2],9,[]],0,[-1,-1,[]],[-1,-2,[],[]],[[-1,1,2,[4,[3]]],[[6,[5]]],[]],[[1,10],11],[[11,12],[[14,[8,13]]]],[[-1,1],8,[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,15,[]],0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,[[1,16,17],[[14,[9,13]]]],0,[[1,2,[18,[3]]],[[14,[9,13]]]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[16,16],[[-1,-2],9,[],[]],[[1,2,19],[[14,[8,13]]]],[[16,16],20],[[16,16],8],[[13,21],22],[[23,21],22],[[7,21],22],[[16,21],[[14,[9,24]]]],[[16,21],[[14,[9,24]]]],[-1,-1,[]],[-1,-1,[]],[-1,-1,[]],[-1,-1,[]],[-1,-1,[]],[25,16],[26,16],[[[9,[-1,27]]],16,[[29,[28]]]],[-1,-1,[]],[17,[[14,[16,30]]]],[[1,2],[[14,[16,13]]]],[1,5],[[16,-1],9,31],0,0,0,[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[16,28],[16,8],[16,8],0,0,[[1,2,[18,[3]]],[[14,[9,13]]]],[[28,27],16],[[16,[4,[3]],17,32],[[14,[1,13]]]],[[16,[4,[3]],17,16,17,32],[[14,[1,13]]]],[[16,[4,[3]],17,17,32],[[14,[1,13]]]],[[[4,[3]]],[[14,[16,30]]]],[[16,16],[[6,[20]]]],[16,27],0,[[16,28],9],[[16,27],9],[-1,-2,[],[]],[16,[[14,[[33,[16]],34]]]],[-1,35,[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,[[14,[-2]]],[],[]],[-1,15,[]],[-1,15,[]],[-1,15,[]],[-1,15,[]],[-1,15,[]],[-1,15,[]],0,[[1,[6,[12]]],9]],"c":[],"p":[[3,"Endpoint",18],[6,"ConnectionId",18],[15,"u8"],[15,"slice"],[15,"usize"],[4,"Option",156],[4,"ConnectionEndReason",18],[15,"bool"],[15,"tuple"],[8,"EndpointEventCallbacks",0],[3,"EndpointHandler",0],[3,"Duration",157],[4,"Error",18],[4,"Result",158],[3,"TypeId",159],[4,"SocketAddr",18],[15,"str"],[3,"Vec",160],[15,"u64"],[4,"Ordering",161],[3,"Formatter",162],[6,"Result",162],[4,"EndpointCloseReason",18],[3,"Error",162],[3,"SocketAddrV4",163],[3,"SocketAddrV6",163],[15,"u16"],[4,"IpAddr",164],[8,"Into",165],[3,"AddrParseError",166],[8,"Hasher",167],[3,"Config",18],[3,"IntoIter",156],[3,"Error",168],[3,"String",169]],"b":[[93,"impl-Debug-for-SocketAddr"],[94,"impl-Display-for-SocketAddr"],[100,"impl-From%3CSocketAddrV4%3E-for-SocketAddr"],[101,"impl-From%3CSocketAddrV6%3E-for-SocketAddr"],[102,"impl-From%3C(I,+u16)%3E-for-SocketAddr"]]}\
}');
if (typeof window !== 'undefined' && window.initSearch) {window.initSearch(searchIndex)};
if (typeof exports !== 'undefined') {exports.searchIndex = searchIndex};

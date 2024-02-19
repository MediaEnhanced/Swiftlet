var searchIndex = JSON.parse('{\
"swiftlet_quic":{"doc":"Providing real-time internet communications using the QUIC …","t":"IDKLLKKKALLKLLKLLLDNNNDNNNNDENNENNNNNNNNLMLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLMMMLLLLLLLLMMLLLLLLLLMLLLLLLLLLLLLLLLLLLLLLMLL","n":["EndpointEventCallbacks","EndpointHandler","background_stream_recv","borrow","borrow_mut","connection_ended","connection_ending_warning","connection_started","endpoint","from","into","main_stream_recv","new","run_event_loop","tick","try_from","try_into","type_id","Config","ConfigCreation","ConnectionClose","ConnectionCreation","ConnectionId","ConnectionNotFound","ConnectionPing","ConnectionRecv","ConnectionSend","Endpoint","Error","IsServer","Randomness","SocketAddr","SocketCreation","SocketRecv","SocketSend","StreamCreation","StreamRecv","StreamSend","V4","V6","add_client_connection","background_recv_first_bytes","background_stream_send","borrow","borrow","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","borrow_mut","borrow_mut","clone","clone","clone_into","clone_into","close_connection","cmp","eq","eq","fmt","fmt","from","from","from","from","from","from","from","from","from_str","get_num_connections","hash","idle_timeout_in_ms","initial_background_recv_size","initial_main_recv_size","into","into","into","into","into","ip","is_ipv4","is_ipv6","keep_alive_timeout","main_recv_first_bytes","main_stream_send","new","new_client","new_client_with_first_connection","new_server","parse_ascii","partial_cmp","port","reliable_stream_buffer","set_ip","set_port","to_owned","to_owned","to_socket_addrs","to_string","try_from","try_from","try_from","try_from","try_from","try_into","try_into","try_into","try_into","try_into","type_id","type_id","type_id","type_id","type_id","unreliable_stream_buffer","update","update_keep_alive_duration"],"q":[[0,"swiftlet_quic"],[18,"swiftlet_quic::endpoint"],[120,"core::option"],[121,"core::time"],[122,"core::result"],[123,"core::any"],[124,"alloc::vec"],[125,"core::cmp"],[126,"core::fmt"],[127,"core::fmt"],[128,"core::net::socket_addr"],[129,"core::convert"],[130,"core::net::parser"],[131,"core::hash"],[132,"core::option"],[133,"alloc::string"]],"d":["Required QUIC Endpoint Handler Event Callback Functions","Main library structure that handles the QUIC Endpoint","Called when there is something to read on the background …","","","Called when a connection has ended and should be cleaned …","Called when a connection is in the process of ending and …","Called when a new connection is started and is application …","QUIC Endpoint Module","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","Called when there is something to read on the main stream.","Create a QUIC Endpoint Handler by giving it an already …","QUIC Endpoint Handler Event Loop","Called when the next tick occurrs based on the tick …","","","","The Endpoint Configuration Structure","Error with the Quic Config Creation","Error closing a connection","Error creating a connection","A Connection ID structure to communicate with the endpoint …","Cannot find connection from Connection ID","Error sending out a PING","Error having a connection process the received data","Error getting send data from a connection","The Quic Endpoint structure","Errors that the QUIC Endpoint can return","Error trying to perform a client Endpoint operation on a …","Error with creating or using the randomness structure / …","An internet socket address, either IPv4 or IPv6.","Error with the UDP socket creation","Error receiving data on the UDP socket","Error sending data on the UDP socket","Error finishing the connection establishment process and …","Error receiving data from the stream","Error sending data on the stream","An IPv4 socket address.","An IPv6 socket address.","Add a connection for a Client Endpoint","The number of bytes to receive on the background stream …","Send data over the background stream","","","","","","","","","","","","","","","Close a connection with a given error code value","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Converts a <code>SocketAddrV6</code> into a <code>SocketAddr::V6</code>.","Returns the argument unchanged.","Converts a <code>SocketAddrV4</code> into a <code>SocketAddr::V4</code>.","Converts a tuple struct (Into&lt;<code>IpAddr</code>&gt;, <code>u16</code>) into a …","","Get the number of connections that the Endpoint is managing","","The quic connection idle timeout in milliseconds.","The initial background stream recieve buffer size.","The initial main stream recieve buffer size.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Returns the IP address associated with this socket address.","Returns <code>true</code> if the IP address in this <code>SocketAddr</code> is an …","Returns <code>true</code> if the IP address in this <code>SocketAddr</code> is an …","The keep alive timeout duration.","The number of bytes to receive on the main stream before …","Send data over the main stream","Creates a new socket address from an IP address and a port …","Create a QUIC Client Endpoint","Create a QUIC Client Endpoint with an initial connection","Create a QUIC Server Endpoint","Parse a socket address from a slice of bytes.","","Returns the port number associated with this socket …","The quic connection bidirectional stream receive buffer …","Changes the IP address associated with this socket address.","Changes the port number associated with this socket …","","","","","","","","","","","","","","","","","","","","The quic connection unidirectional stream receive buffer …","A way to update the Connection ID","Update the keep alive duration time"],"i":[0,0,9,10,10,9,9,9,0,10,10,9,10,10,9,10,10,10,0,12,12,12,0,12,12,12,12,0,0,12,12,0,12,12,12,12,12,12,15,15,1,29,1,29,1,2,12,15,29,1,2,12,15,2,15,2,15,1,15,2,15,15,15,29,1,2,12,15,15,15,15,15,1,15,29,29,29,29,1,2,12,15,15,15,15,29,29,1,15,1,1,1,15,15,15,29,15,15,2,15,15,15,29,1,2,12,15,29,1,2,12,15,29,1,2,12,15,29,2,1],"f":[0,0,[[-1,1,2,[4,[3]]],[[6,[5]]],[]],[-1,-2,[],[]],[-1,-2,[],[]],[[-1,1,2,5],7,[]],[[-1,1,2],8,[]],[[-1,1,2],8,[]],0,[-1,-1,[]],[-1,-2,[],[]],[[-1,1,2,[4,[3]]],[[6,[5]]],[]],[[1,9],10],[[10,11],[[13,[[6,[1]],12]]]],[[-1,1],7,[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,14,[]],0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,[[1,15,16],[[13,[8,12]]]],0,[[1,2,[17,[3]]],[[13,[[8,[18,18]],12]]]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[2,2],[15,15],[[-1,-2],8,[],[]],[[-1,-2],8,[],[]],[[1,2,18],[[13,[7,12]]]],[[15,15],19],[[2,2],7],[[15,15],7],[[15,20],[[13,[8,21]]]],[[15,20],[[13,[8,21]]]],[-1,-1,[]],[-1,-1,[]],[-1,-1,[]],[-1,-1,[]],[22,15],[-1,-1,[]],[23,15],[[[8,[-1,24]]],15,[[26,[25]]]],[16,[[13,[15,27]]]],[1,5],[[15,-1],8,28],0,0,0,[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[-1,-2,[],[]],[15,25],[15,7],[15,7],0,0,[[1,2,[17,[3]]],[[13,[[8,[18,18]],12]]]],[[25,24],15],[[15,[4,[3]],16,29],[[13,[1,12]]]],[[15,[4,[3]],16,15,16,29],[[13,[1,12]]]],[[15,[4,[3]],16,16,29],[[13,[1,12]]]],[[[4,[3]]],[[13,[15,27]]]],[[15,15],[[6,[19]]]],[15,24],0,[[15,25],8],[[15,24],8],[-1,-2,[],[]],[-1,-2,[],[]],[15,[[13,[[30,[15]],31]]]],[-1,32,[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,[[13,[-2]]],[],[]],[-1,14,[]],[-1,14,[]],[-1,14,[]],[-1,14,[]],[-1,14,[]],0,[[2,2],8],[[1,[6,[11]]],8]],"c":[],"p":[[3,"Endpoint",18],[3,"ConnectionId",18],[15,"u8"],[15,"slice"],[15,"usize"],[4,"Option",120],[15,"bool"],[15,"tuple"],[8,"EndpointEventCallbacks",0],[3,"EndpointHandler",0],[3,"Duration",121],[4,"Error",18],[4,"Result",122],[3,"TypeId",123],[4,"SocketAddr",18],[15,"str"],[3,"Vec",124],[15,"u64"],[4,"Ordering",125],[3,"Formatter",126],[3,"Error",126],[3,"SocketAddrV6",127],[3,"SocketAddrV4",127],[15,"u16"],[4,"IpAddr",128],[8,"Into",129],[3,"AddrParseError",130],[8,"Hasher",131],[3,"Config",18],[3,"IntoIter",120],[3,"Error",132],[3,"String",133]],"b":[[61,"impl-Debug-for-SocketAddr"],[62,"impl-Display-for-SocketAddr"],[67,"impl-From%3CSocketAddrV6%3E-for-SocketAddr"],[69,"impl-From%3CSocketAddrV4%3E-for-SocketAddr"],[70,"impl-From%3C(I,+u16)%3E-for-SocketAddr"]]}\
}');
if (typeof window !== 'undefined' && window.initSearch) {window.initSearch(searchIndex)};
if (typeof exports !== 'undefined') {exports.searchIndex = searchIndex};

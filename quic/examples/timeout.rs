//Media Enhanced Swiftlet Networking Mio Sleep Recv Timeout Bad
//MIT License
//Copyright (c) 2024 Jared Loewenthal
//
//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files (the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions:
//
//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.

use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    println!("Mio Recv Timeout Example");

    let bind_address = std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
        std::net::Ipv6Addr::UNSPECIFIED,
        9001,
        0,
        0,
    ));

    let mut socket = mio::net::UdpSocket::bind(bind_address)?;

    let mut poll = mio::Poll::new()?;

    poll.registry()
        .register(&mut socket, mio::Token(0), mio::Interest::READABLE)?;

    let mut events = mio::Events::with_capacity(1024);

    for timeout_ms in 1..30 {
        let timeout_duration = Duration::from_millis(timeout_ms);
        println!("Mio Recv Timeout Duration Test: {:?}", timeout_duration);
        for _ in 0..10 {
            let before_instant = Instant::now();
            poll.poll(&mut events, Some(timeout_duration))?;
            let after_instant = Instant::now();

            let dur = after_instant - before_instant;
            println!("Timeout Duration: {:?}", dur);
        }
    }

    println!("Mio Recv Timeout Example Ended!");

    Ok(())
}

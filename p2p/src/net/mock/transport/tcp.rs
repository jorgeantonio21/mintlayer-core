// Copyright (c) 2022 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://github.com/mintlayer/mintlayer-core/blob/master/LICENSE
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::net::SocketAddr;

use async_trait::async_trait;
use tokio::net::{TcpListener, TcpStream};

use crate::{
    net::mock::{
        transport::{MockListener, MockStream, MockTransport},
        types::Message,
    },
    Result,
};

#[derive(Debug)]
pub struct TcpMockTransport {}

#[async_trait]
impl MockTransport for TcpMockTransport {
    type Address = SocketAddr;
    type Listener = TcpListener;
    type Stream = TcpStream;

    async fn bind(address: Self::Address) -> Result<Self::Listener> {
        TcpListener::bind(address).await.map_err(Into::into)
    }

    // async fn connect(address: Self::Address) -> Result<Self::Connection> {
    //     //TcpStream::connect(address)
    //     todo!()
    // }
}

#[async_trait]
impl MockListener<TcpStream, SocketAddr> for TcpListener {
    async fn accept(&mut self) -> Result<(TcpStream, SocketAddr)> {
        todo!()
    }
}

#[async_trait]
impl MockStream for TcpStream {
    async fn send(&mut self, msg: Message) -> Result<()> {
        todo!()
    }

    async fn recv(&mut self) -> Result<Option<Message>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //     use crate::{message::*, net::mock::types};
    //     use tokio::net::TcpListener;
    //
    //     #[tokio::test]
    //     async fn test_mock_socket_send_recv() {
    //         let addr: std::net::SocketAddr = "[::1]:0".parse().unwrap();
    //         let server = TcpListener::bind(addr).await.unwrap();
    //         let peer_fut = TcpStream::connect(server.local_addr().unwrap());
    //
    //         let (res1, res2) = tokio::join!(server.accept(), peer_fut);
    //         let mut server_socket = MockSocket::new(res1.unwrap().0);
    //         let mut peer_socket = MockSocket::new(res2.unwrap());
    //
    //         let msg = Message::Request {
    //             request_id: types::MockRequestId::new(1337u64),
    //             request: Request::BlockListRequest(BlockListRequest::new(vec![])),
    //         };
    //         peer_socket.send(msg.clone()).await.unwrap();
    //
    //         assert_eq!(server_socket.recv().await.unwrap().unwrap(), msg);
    //     }
}

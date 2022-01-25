// Copyright (c) 2021 Protocol Labs
// Copyright (c) 2022 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://spdx.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Author(s): A. Altonen
use crate::error;
use libp2p::{
    streaming::{IdentityCodec, StreamHandle, Streaming, StreamingEvent},
    swarm::NegotiatedSubstream,
    Multiaddr, NetworkBehaviour, PeerId,
};
use tokio::sync::oneshot;

pub enum Command {
    /// Start listening on the network interface specified by `addr`
    Listen {
        addr: Multiaddr,
        response: oneshot::Sender<error::Result<()>>,
    },
    /// Connect to a remote peer at address `peer_addr` whose PeerId is `peer_id`
    Dial {
        peer_id: PeerId,
        peer_addr: Multiaddr,
        response: oneshot::Sender<error::Result<()>>,
    },
    /// Open a bidirectional data stream to a remote peer
    ///
    /// Before opening a stream, connection must've been established with the peer
    /// and the peer's identity is signaled using `peer_id` argument
    OpenStream {
        peer_id: PeerId,
        response: oneshot::Sender<error::Result<StreamHandle<NegotiatedSubstream>>>,
    },
}

pub enum Event {}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ComposedEvent")]
pub struct ComposedBehaviour {
    pub streaming: Streaming<IdentityCodec>,
}

#[derive(Debug)]
pub enum ComposedEvent {
    StreamingEvent(StreamingEvent<IdentityCodec>),
}

impl From<StreamingEvent<IdentityCodec>> for ComposedEvent {
    fn from(event: StreamingEvent<IdentityCodec>) -> Self {
        ComposedEvent::StreamingEvent(event)
    }
}

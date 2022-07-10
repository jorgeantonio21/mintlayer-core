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
use crate::{
    net::{self, libp2p::Libp2pService, ConnectivityService},
    swarm::tests::make_peer_manager,
};
use common::chain::config;
use libp2p::Multiaddr;
use std::sync::Arc;

// ban peer whose connected to us
#[tokio::test]
async fn ban_connected_peer() {
    let addr: Multiaddr = test_utils::make_address("/ip6/::1/tcp/");
    let config = Arc::new(config::create_mainnet());
    let mut swarm1 = make_peer_manager::<Libp2pService>(addr, Arc::clone(&config)).await;
    let mut swarm2 =
        make_peer_manager::<Libp2pService>(test_utils::make_address("/ip6/::1/tcp/"), config).await;

    let (_conn1_res, conn2_res) = tokio::join!(
        swarm1.handle.connect(swarm2.handle.local_addr().clone()),
        swarm2.handle.poll_next(),
    );

    if let Ok(net::types::ConnectivityEvent::IncomingConnection { peer_info, addr }) = conn2_res {
        swarm2.accept_inbound_connection(addr, peer_info).await.unwrap();
    }

    let peer_id = *swarm1.handle_mut().peer_id();
    assert_eq!(swarm2.adjust_peer_score(peer_id, 1000).await, Ok(()));
    assert!(!swarm2.validate_peer_id(&peer_id));
    assert!(std::matches!(
        swarm2.handle_mut().poll_next().await,
        Ok(net::types::ConnectivityEvent::ConnectionClosed { .. })
    ));
}

#[tokio::test]
async fn banned_peer_attempts_to_connect() {
    let addr: Multiaddr = test_utils::make_address("/ip6/::1/tcp/");
    let config = Arc::new(config::create_mainnet());
    let mut swarm1 = make_peer_manager::<Libp2pService>(addr, Arc::clone(&config)).await;
    let mut swarm2 =
        make_peer_manager::<Libp2pService>(test_utils::make_address("/ip6/::1/tcp/"), config).await;

    let (_conn1_res, conn2_res) = tokio::join!(
        swarm1.handle.connect(swarm2.handle.local_addr().clone()),
        swarm2.handle.poll_next(),
    );

    if let Ok(net::types::ConnectivityEvent::IncomingConnection { peer_info, addr }) = conn2_res {
        swarm2.accept_inbound_connection(addr, peer_info).await.unwrap();
    }

    let peer_id = *swarm1.handle_mut().peer_id();
    assert_eq!(swarm2.adjust_peer_score(peer_id, 1000).await, Ok(()));
    assert!(!swarm2.validate_peer_id(&peer_id));
    assert!(std::matches!(
        swarm2.handle_mut().poll_next().await,
        Ok(net::types::ConnectivityEvent::ConnectionClosed { .. })
    ));

    // try to restablish connection, it timeouts because it's rejected in the backend
    let addr = swarm2.handle.local_addr().clone();
    tokio::spawn(async move { swarm1.handle.connect(addr).await });

    tokio::select! {
        _event = swarm2.handle.poll_next() => {
            panic!("did not expect event, received {:?}", _event)
        },
        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
    }
}

// attempt to connect to banned peer
#[tokio::test]
async fn connect_to_banned_peer() {
    let addr: Multiaddr = test_utils::make_address("/ip6/::1/tcp/");
    let config = Arc::new(config::create_mainnet());
    let mut swarm1 = make_peer_manager::<Libp2pService>(addr, Arc::clone(&config)).await;
    let mut swarm2 =
        make_peer_manager::<Libp2pService>(test_utils::make_address("/ip6/::1/tcp/"), config).await;

    let (_conn1_res, conn2_res) = tokio::join!(
        swarm1.handle.connect(swarm2.handle.local_addr().clone()),
        swarm2.handle.poll_next(),
    );

    if let Ok(net::types::ConnectivityEvent::IncomingConnection { peer_info, addr }) = conn2_res {
        swarm2.accept_inbound_connection(addr, peer_info).await.unwrap();
    }

    let peer_id = *swarm1.handle_mut().peer_id();
    assert_eq!(swarm2.adjust_peer_score(peer_id, 1000).await, Ok(()));
    assert!(!swarm2.validate_peer_id(&peer_id));
    assert!(std::matches!(
        swarm2.handle_mut().poll_next().await,
        Ok(net::types::ConnectivityEvent::ConnectionClosed { .. })
    ));

    println!(
        "{:?}",
        swarm2.handle.connect(swarm1.handle.local_addr().clone()).await
    );
}

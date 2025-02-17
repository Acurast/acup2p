package com.acurast.p2p

import uniffi.acup2p.NodeId
import uniffi.acup2p.PublicKey
import uniffi.acup2p.nodeIdFromPublicKey

public fun NodeId.Companion.Ed25519(publicKey: ByteArray): NodeId =
    fromPublicKey(PublicKey.Ed25519(publicKey))

internal fun NodeId.Companion.fromPublicKey(publicKey: PublicKey): NodeId =
    nodeIdFromPublicKey(publicKey)
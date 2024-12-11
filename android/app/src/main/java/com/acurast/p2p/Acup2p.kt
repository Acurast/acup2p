package com.acurast.p2p

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.ClosedReceiveChannelException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.launch
import uniffi.acup2p.Config
import uniffi.acup2p.Event
import uniffi.acup2p.Identity
import uniffi.acup2p.InboundProtocolRequest
import uniffi.acup2p.Intent
import uniffi.acup2p.NodeId
import uniffi.acup2p.OutboundProtocolMessage
import uniffi.acup2p.OutboundProtocolRequest
import uniffi.acup2p.OutboundProtocolResponse
import uniffi.acup2p.SecretKey
import uniffi.acup2p.bind
import uniffi.acup2p.defaultConfig
import kotlin.coroutines.CoroutineContext

public class Acup2p(coroutineContext: CoroutineContext, config: Config = Config.Default) {
    private val coroutineScope = CoroutineScope(coroutineContext + SupervisorJob(coroutineContext[Job]))
    private val handler: Handler = Handler()

    public val events: Flow<Event>
        get() = handler.events

    init {
        coroutineScope.launch {
            try {
                bind(handler, config)
            } catch (e: CancellationException) { /* no action */ }
        }
    }

    public suspend fun connect(nodes: List<NodeId>) {
        handler.intents.send(Intent.Connect(nodes))
    }

    public suspend fun disconnect(nodes: List<NodeId>) {
        handler.intents.send(Intent.Disconnect(nodes))
    }

    public suspend fun sendMessage(request: OutboundProtocolRequest, nodes: List<NodeId>) {
        sendMessage(OutboundProtocolMessage.Request(request), nodes)
    }

    public suspend fun sendMessage(response: OutboundProtocolResponse, nodes: List<NodeId>) {
        sendMessage(OutboundProtocolMessage.Response(response), nodes)
    }

    public suspend fun sendMessage(message: OutboundProtocolMessage, nodes: List<NodeId>) {
        handler.intents.send(Intent.SendMessage(message, nodes))
    }

    public suspend fun close() {
        handler.intents.send(Intent.Close)
        coroutineScope.cancel()
    }

    private class Handler : uniffi.acup2p.Handler {
        private val _events: MutableSharedFlow<Event> = MutableSharedFlow(replay = BUFFER_CAPACITY)
        val events: SharedFlow<Event>
            get() = _events.asSharedFlow()

        val intents: Channel<Intent> = Channel(Channel.UNLIMITED)

        override suspend fun onEvent(event: Event) {
            _events.emit(event)
        }

        override suspend fun nextIntent(): Intent? =
            try {
                intents.receive()
            } catch (e: ClosedReceiveChannelException) {
                null
            } catch (e: CancellationException) {
                null
            }

        companion object {
            private const val BUFFER_CAPACITY = 1024
        }
    }
}

public fun CoroutineScope.Acup2p(config: Config = Config.Default): Acup2p = Acup2p(coroutineContext, config)

public val Config.Companion.Default: Config
    get() = defaultConfig()

public fun Identity.Companion.Ed25519(secretKey: ByteArray): Identity.Keypair =
    Identity.Keypair(SecretKey.Ed25519(secretKey))

public fun OutboundProtocolResponse.Companion.fromRequest(request: InboundProtocolRequest, bytes: ByteArray): OutboundProtocolResponse =
    OutboundProtocolResponse(request.protocol, bytes, request.id)
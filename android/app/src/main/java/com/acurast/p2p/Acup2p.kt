package com.acurast.p2p

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.merge
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
import uniffi.acup2p.StreamConsumer
import uniffi.acup2p.StreamProducer
import uniffi.acup2p.bind
import uniffi.acup2p.defaultConfig
import kotlin.coroutines.CoroutineContext

public class Acup2p(coroutineContext: CoroutineContext, config: Config = Config.Default) {
    private val coroutineScope = CoroutineScope(coroutineContext + SupervisorJob(coroutineContext[Job]))
    private val handler: Handler = Handler()
    private val incomingStreamHandlers: List<IncomingStreamHandler> =
        config.streamProtocols.map { IncomingStreamHandler(it) }

    public val events: Flow<Event>
        get() = handler.events

    public val incomingStreams: Flow<Stream>
        get() = incomingStreamHandlers
            .map { it.streams }
            .merge()

    init {
        coroutineScope.launch {
            try {
                bind(handler, incomingStreamHandlers, config)
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

    public suspend fun openOutgoingStream(protocol: String, node: NodeId): Stream {
        val consumer = Stream.Consumer()
        val producer = Stream.Producer()
        val stream = Stream(protocol, node, consumer, producer)
        handler.intents.send(Intent.OpenOutgoingStream(protocol, node, producer, consumer))

        return stream
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
            intents.receiveIfActive()

        companion object {
            private const val BUFFER_CAPACITY = 1024
        }
    }

    private class IncomingStreamHandler(private val protocol: String) : uniffi.acup2p.IncomingStreamHandler {
        private val _streams: MutableSharedFlow<Stream> = MutableSharedFlow()
        val streams: SharedFlow<Stream>
            get() = _streams.asSharedFlow()

        private var nextStream: Stream? = null

        override fun protocol(): String = protocol
        override fun consumer(): StreamConsumer = nextStream?.consumer ?: failWithStreamNotInitialized()
        override fun producer(): StreamProducer = nextStream?.producer ?: failWithStreamNotInitialized()

        override suspend fun createStream(node: NodeId) {
            nextStream = Stream(protocol, node, Stream.Consumer(), Stream.Producer())
        }

        override suspend fun finalizeStream() {
            nextStream?.let { _streams.emit(it) }
        }

        private fun failWithStreamNotInitialized(): Nothing =
            throw IllegalStateException("Next stream not initialized, call `create_stream` first")
    }
}

public fun CoroutineScope.Acup2p(config: Config = Config.Default): Acup2p = Acup2p(coroutineContext, config)

public val Config.Companion.Default: Config
    get() = defaultConfig()

public fun Identity.Companion.Ed25519(secretKey: ByteArray): Identity.Keypair =
    Identity.Keypair(SecretKey.Ed25519(secretKey))

public fun OutboundProtocolResponse.Companion.fromRequest(request: InboundProtocolRequest, bytes: ByteArray): OutboundProtocolResponse =
    OutboundProtocolResponse(request.protocol, bytes, request.id)
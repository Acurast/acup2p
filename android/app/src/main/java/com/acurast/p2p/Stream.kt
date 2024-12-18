package com.acurast.p2p

import kotlinx.coroutines.channels.Channel
import uniffi.acup2p.NodeId
import uniffi.acup2p.StreamRead
import uniffi.acup2p.StreamWrite

public interface ReadBytes {
    public suspend fun read(n: UInt): ByteArray?
    public suspend fun close()
}

public interface WriteBytes {
    public suspend fun write(bytes: ByteArray): UInt
    public suspend fun close()
}

public class Stream internal constructor(
    public val protocol: String,
    public val node: NodeId,
    private val consumer: Consumer,
    private val producer: Producer,
) : ReadBytes by consumer, WriteBytes by producer {

    override suspend fun close() {
        consumer.close()
        producer.close()
    }

    internal class Consumer : uniffi.acup2p.StreamConsumer, ReadBytes {
        private val read: Channel<UInt> = Channel(Channel.UNLIMITED)
        private val bytes: Channel<Result<ByteArray>> = Channel(Channel.UNLIMITED)

        override suspend fun read(n: UInt): ByteArray? {
            read.send(n)

            return bytes.receiveIfActive()?.getOrThrow()
        }

        override suspend fun close() {
            read.close()
        }

        override suspend fun nextRead(): UInt? =
            read.receiveIfActive()

        override suspend fun onBytes(read: StreamRead) {
            when (read) {
                StreamRead.Eos -> bytes.close()
                is StreamRead.Err -> bytes.send(Result.failure(StreamException(read.v1)))
                is StreamRead.Ok -> bytes.send(Result.success(read.v1))
            }
        }
    }

    internal class Producer : uniffi.acup2p.StreamProducer, WriteBytes {
        private val bytes: Channel<ByteArray> = Channel(Channel.UNLIMITED)
        private val result: Channel<Result<Unit>> = Channel(Channel.UNLIMITED)

        override suspend fun write(bytes: ByteArray): UInt {
            this.bytes.send(bytes)

            return result.receiveIfActive()?.getOrThrow()?.let { bytes.size.toUInt() } ?: 0U
        }

        override suspend fun close() {
            bytes.close()
        }

        override suspend fun nextBytes(): ByteArray? =
            bytes.receiveIfActive()

        override suspend fun onFinished(write: StreamWrite) {
            when (write) {
                StreamWrite.Eos -> result.close()
                is StreamWrite.Err -> result.send(Result.failure(StreamException(write.v1)))
                StreamWrite.Ok -> result.send(Result.success(Unit))
            }
        }
    }

    public companion object
}

public class StreamException(message: String) : Exception(message)
package com.acurast.p2p

import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.ClosedReceiveChannelException
import kotlin.coroutines.cancellation.CancellationException

internal suspend fun <T> Channel<T>.receiveIfActive(): T? =
    try {
        receive()
    } catch (e: ClosedReceiveChannelException) {
        null
    } catch (e: CancellationException) {
        null
    }

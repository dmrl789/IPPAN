package org.ippan.wallet.network

import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class IppanApiClientTest {

    private lateinit var primary: MockWebServer
    private lateinit var secondary: MockWebServer

    @Before
    fun setUp() {
        primary = MockWebServer().apply { start() }
        secondary = MockWebServer().apply { start() }
    }

    @After
    fun tearDown() {
        primary.shutdown()
        secondary.shutdown()
    }

    @Test
    fun `returns balance from the first healthy node`() = runBlocking {
        primary.enqueue(
            MockResponse()
                .setResponseCode(200)
                .setBody(
                    """
                    {
                      "address": "ippan_address",
                      "balance": 125.4,
                      "currency": "IPP",
                      "lastUpdated": "2025-10-17T12:00:00Z"
                    }
                    """.trimIndent()
                )
        )

        val client = IppanApiClient(listOf(primary.url("/").toString(), secondary.url("/").toString()))

        val result = client.getBalance("ippan_address")

        assertTrue(result.isSuccess)
        assertEquals(1, primary.requestCount)
        assertEquals(0, secondary.requestCount)
        val response = result.getOrThrow()
        assertEquals("ippan_address", response.address)
        assertEquals(125.4, response.balance, 0.0)
        assertEquals("IPP", response.currency)
    }

    @Test
    fun `fails over to the next node when the primary returns server error`() = runBlocking {
        primary.enqueue(MockResponse().setResponseCode(500))
        secondary.enqueue(
            MockResponse()
                .setResponseCode(200)
                .setBody(
                    """
                    {
                      "address": "ippan_address",
                      "balance": 10.0,
                      "currency": "IPP",
                      "lastUpdated": "2025-10-17T12:05:00Z"
                    }
                    """.trimIndent()
                )
        )

        val client = IppanApiClient(listOf(primary.url("/").toString(), secondary.url("/").toString()))

        val result = client.getBalance("ippan_address")

        assertTrue(result.isSuccess)
        assertEquals(1, primary.requestCount)
        assertEquals(1, secondary.requestCount)
        assertEquals(secondary.url("/").toString().trimEnd('/'), client.activeNode)
    }

    @Test
    fun `returns failure when all nodes are unreachable`() = runBlocking {
        primary.enqueue(MockResponse().setResponseCode(500))
        secondary.enqueue(MockResponse().setSocketPolicy(okhttp3.mockwebserver.SocketPolicy.DISCONNECT_AT_START))

        val client = IppanApiClient(listOf(primary.url("/").toString(), secondary.url("/").toString()))

        val result = client.getBalance("ippan_address")

        assertTrue(result.isFailure)
    }
}

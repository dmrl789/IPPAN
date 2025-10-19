package org.ippan.wallet.network

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * IPPAN API client for blockchain operations
 */
class IppanApiClient(
    baseUrls: List<String>,
    private val timeoutSeconds: Long = 30
) {
    private val nodes: List<String> = if (baseUrls.isNotEmpty()) {
        baseUrls.map { it.trim().trimEnd('/') }
    } else {
        listOf("https://api.ippan.net")
    }
    @Volatile
    private var activeNodeIndex: Int = 0

    val activeNode: String
        get() = nodes[activeNodeIndex]

    private val client = OkHttpClient.Builder()
        .connectTimeout(timeoutSeconds, TimeUnit.SECONDS)
        .readTimeout(timeoutSeconds, TimeUnit.SECONDS)
        .writeTimeout(timeoutSeconds, TimeUnit.SECONDS)
        .build()

    private val json = Json {
        ignoreUnknownKeys = true
        coerceInputValues = true
    }

    private val jsonMediaType = "application/json".toMediaType()

    /**
     * Get account balance for an address
     */
    suspend fun getBalance(address: String): Result<BalanceResponse> = executeWithFailover(
        requestBuilder = { base ->
            Request.Builder()
                .url(buildUrl(base, "/api/balance/$address"))
                .header("Accept", jsonMediaType.toString())
                .get()
                .build()
        },
        parser = { body -> json.decodeFromString(body) }
    )

    /**
     * Get transaction history for an address
     */
    suspend fun getTransactions(address: String, limit: Int = 50): Result<List<TransactionResponse>> = executeWithFailover(
        requestBuilder = { base ->
            Request.Builder()
                .url(buildUrl(base, "/api/transactions/$address?limit=$limit"))
                .header("Accept", jsonMediaType.toString())
                .get()
                .build()
        },
        parser = { body -> json.decodeFromString(body) }
    )

    /**
     * Submit a signed transaction
     */
    suspend fun submitTransaction(transaction: SignedTransactionRequest): Result<TransactionSubmissionResponse> {
        val jsonBody = json.encodeToString(SignedTransactionRequest.serializer(), transaction)
        val requestBody = jsonBody.toRequestBody(jsonMediaType)
        return executeWithFailover(
            requestBuilder = { base ->
                Request.Builder()
                    .url(buildUrl(base, "/api/transactions"))
                    .header("Accept", jsonMediaType.toString())
                    .post(requestBody)
                    .build()
            },
            parser = { body -> json.decodeFromString(body) }
        )
    }

    /**
     * Get network status and health
     */
    suspend fun getNetworkStatus(): Result<NetworkStatusResponse> = executeWithFailover(
        requestBuilder = { base ->
            Request.Builder()
                .url(buildUrl(base, "/api/status"))
                .header("Accept", jsonMediaType.toString())
                .get()
                .build()
        },
        parser = { body -> json.decodeFromString(body) }
    )

    /**
     * Get current gas price for transaction fees
     */
    suspend fun getGasPrice(): Result<GasPriceResponse> = executeWithFailover(
        requestBuilder = { base ->
            Request.Builder()
                .url(buildUrl(base, "/api/gas-price"))
                .header("Accept", jsonMediaType.toString())
                .get()
                .build()
        },
        parser = { body -> json.decodeFromString(body) }
    )

    val availableNodes: List<String>
        get() = nodes.toList()

    private suspend fun <T> executeWithFailover(
        requestBuilder: (String) -> Request,
        parser: (String) -> T
    ): Result<T> = withContext(Dispatchers.IO) {
        var lastError: Throwable? = null
        for (offset in nodes.indices) {
            val index = (activeNodeIndex + offset) % nodes.size
            val base = nodes[index]
            try {
                client.newCall(requestBuilder(base)).execute().use { response ->
                    when {
                        response.isSuccessful -> {
                            val body = response.body?.string() ?: throw IOException("Empty response body")
                            val parsed = parser(body)
                            activeNodeIndex = index
                            return@withContext Result.success(parsed)
                        }
                        response.shouldRetry() -> {
                            lastError = IOException("HTTP ${response.code}: ${response.message}")
                            return@use
                        }
                        else -> {
                            return@withContext Result.failure(IOException("HTTP ${response.code}: ${response.message}"))
                        }
                    }
                }
            } catch (e: Exception) {
                lastError = e
            }
        }
        Result.failure(lastError ?: IOException("Unable to reach any IPPAN node"))
    }

    private fun buildUrl(base: String, path: String): String {
        val sanitizedBase = base.trimEnd('/')
        val sanitizedPath = if (path.startsWith("/")) path else "/$path"
        return sanitizedBase + sanitizedPath
    }

    private fun Response.shouldRetry(): Boolean {
        return code in 500..599 || code in RETRYABLE_STATUS_CODES
    }

    companion object {
        private val RETRYABLE_STATUS_CODES = setOf(408, 425, 429)
    }
}

// API Response Models
@Serializable
data class BalanceResponse(
    val address: String,
    val balance: Double,
    val currency: String,
    val lastUpdated: String
)

@Serializable
data class TransactionResponse(
    val id: String,
    val from: String,
    val to: String,
    val amount: Double,
    val currency: String,
    val timestamp: String,
    val status: String,
    val blockHeight: Long?,
    val gasUsed: Long?,
    val gasPrice: Long?
)

@Serializable
data class SignedTransactionRequest(
    val from: String,
    val to: String,
    val amount: Double,
    val currency: String,
    val nonce: Long,
    val gasPrice: Long,
    val gasLimit: Long,
    val signature: String,
    val publicKey: String
)

@Serializable
data class TransactionSubmissionResponse(
    val transactionId: String,
    val status: String,
    val message: String?
)

@Serializable
data class NetworkStatusResponse(
    val status: String,
    val blockHeight: Long,
    val networkId: String,
    val version: String,
    val peers: Int
)

@Serializable
data class GasPriceResponse(
    val gasPrice: Long,
    val currency: String,
    val timestamp: String
)

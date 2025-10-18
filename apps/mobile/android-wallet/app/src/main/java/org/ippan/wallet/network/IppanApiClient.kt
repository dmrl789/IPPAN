package org.ippan.wallet.network

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.time.Instant
import java.util.concurrent.TimeUnit

/**
 * IPPAN API client for blockchain operations
 */
class IppanApiClient(
    private val baseUrl: String,
    private val timeoutSeconds: Long = 30
) {
    private val client = OkHttpClient.Builder()
        .connectTimeout(timeoutSeconds, TimeUnit.SECONDS)
        .readTimeout(timeoutSeconds, TimeUnit.SECONDS)
        .writeTimeout(timeoutSeconds, TimeUnit.SECONDS)
        .build()

    private val json = Json {
        ignoreUnknownKeys = true
        coerceInputValues = true
    }

    /**
     * Get account balance for an address
     */
    suspend fun getBalance(address: String): Result<BalanceResponse> = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url("$baseUrl/api/balance/$address")
                .get()
                .build()

            val response = client.newCall(request).execute()
            if (response.isSuccessful) {
                val body = response.body?.string() ?: throw IOException("Empty response body")
                val balanceResponse = json.decodeFromString<BalanceResponse>(body)
                Result.success(balanceResponse)
            } else {
                Result.failure(IOException("HTTP ${response.code}: ${response.message}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get transaction history for an address
     */
    suspend fun getTransactions(address: String, limit: Int = 50): Result<List<TransactionResponse>> = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url("$baseUrl/api/transactions/$address?limit=$limit")
                .get()
                .build()

            val response = client.newCall(request).execute()
            if (response.isSuccessful) {
                val body = response.body?.string() ?: throw IOException("Empty response body")
                val transactions = json.decodeFromString<List<TransactionResponse>>(body)
                Result.success(transactions)
            } else {
                Result.failure(IOException("HTTP ${response.code}: ${response.message}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Submit a signed transaction
     */
    suspend fun submitTransaction(transaction: SignedTransactionRequest): Result<TransactionSubmissionResponse> = withContext(Dispatchers.IO) {
        try {
            val jsonBody = json.encodeToString(SignedTransactionRequest.serializer(), transaction)
            val requestBody = jsonBody.toRequestBody("application/json".toMediaType())
            
            val request = Request.Builder()
                .url("$baseUrl/api/transactions")
                .post(requestBody)
                .build()

            val response = client.newCall(request).execute()
            if (response.isSuccessful) {
                val body = response.body?.string() ?: throw IOException("Empty response body")
                val submissionResponse = json.decodeFromString<TransactionSubmissionResponse>(body)
                Result.success(submissionResponse)
            } else {
                Result.failure(IOException("HTTP ${response.code}: ${response.message}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get network status and health
     */
    suspend fun getNetworkStatus(): Result<NetworkStatusResponse> = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url("$baseUrl/api/status")
                .get()
                .build()

            val response = client.newCall(request).execute()
            if (response.isSuccessful) {
                val body = response.body?.string() ?: throw IOException("Empty response body")
                val statusResponse = json.decodeFromString<NetworkStatusResponse>(body)
                Result.success(statusResponse)
            } else {
                Result.failure(IOException("HTTP ${response.code}: ${response.message}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get current gas price for transaction fees
     */
    suspend fun getGasPrice(): Result<GasPriceResponse> = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url("$baseUrl/api/gas-price")
                .get()
                .build()

            val response = client.newCall(request).execute()
            if (response.isSuccessful) {
                val body = response.body?.string() ?: throw IOException("Empty response body")
                val gasPriceResponse = json.decodeFromString<GasPriceResponse>(body)
                Result.success(gasPriceResponse)
            } else {
                Result.failure(IOException("HTTP ${response.code}: ${response.message}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
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

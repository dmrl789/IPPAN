package org.ippan.wallet.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * Service for fetching real-time fiat conversion rates
 */
class FiatConversionService {
    
    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build()
    
    private val json = Json {
        ignoreUnknownKeys = true
        coerceInputValues = true
    }
    
    /**
     * Get current exchange rate for IPPAN token
     */
    suspend fun getExchangeRate(
        fromCurrency: String = "IPP",
        toCurrency: String = "USD"
    ): Result<ExchangeRate> = withContext(Dispatchers.IO) {
        try {
            // Try CoinGecko API first (free tier)
            val coinGeckoResult = tryCoinGecko(fromCurrency, toCurrency)
            if (coinGeckoResult.isSuccess) {
                return@withContext coinGeckoResult
            }
            
            // Fallback to CoinMarketCap API
            val cmcResult = tryCoinMarketCap(fromCurrency, toCurrency)
            if (cmcResult.isSuccess) {
                return@withContext cmcResult
            }
            
            // Final fallback to mock data
            Result.success(ExchangeRate(
                from = fromCurrency,
                to = toCurrency,
                rate = 0.1, // Placeholder rate
                timestamp = System.currentTimeMillis(),
                source = "fallback"
            ))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    private suspend fun tryCoinGecko(fromCurrency: String, toCurrency: String): Result<ExchangeRate> {
        return try {
            val request = Request.Builder()
                .url("https://api.coingecko.com/api/v3/simple/price?ids=ippan&vs_currencies=${toCurrency.lowercase()}")
                .build()
            
            client.newCall(request).execute().use { response ->
                if (response.isSuccessful) {
                    val body = response.body?.string() ?: throw IOException("Empty response")
                    val data = json.decodeFromString<CoinGeckoResponse>(body)
                    val rate = data.ippan?.get(toCurrency.lowercase()) ?: throw IOException("Rate not found")
                    
                    Result.success(ExchangeRate(
                        from = fromCurrency,
                        to = toCurrency,
                        rate = rate,
                        timestamp = System.currentTimeMillis(),
                        source = "coingecko"
                    ))
                } else {
                    Result.failure(IOException("HTTP ${response.code}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    private suspend fun tryCoinMarketCap(fromCurrency: String, toCurrency: String): Result<ExchangeRate> {
        return try {
            // Note: CoinMarketCap requires API key in production
            val request = Request.Builder()
                .url("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?symbol=${fromCurrency}&convert=${toCurrency}")
                .addHeader("X-CMC_PRO_API_KEY", "your-api-key-here") // Replace with real API key
                .build()
            
            client.newCall(request).execute().use { response ->
                if (response.isSuccessful) {
                    val body = response.body?.string() ?: throw IOException("Empty response")
                    val data = json.decodeFromString<CoinMarketCapResponse>(body)
                    val rate = data.data?.values?.firstOrNull()?.quote?.get(toCurrency)?.price
                        ?: throw IOException("Rate not found")
                    
                    Result.success(ExchangeRate(
                        from = fromCurrency,
                        to = toCurrency,
                        rate = rate,
                        timestamp = System.currentTimeMillis(),
                        source = "coinmarketcap"
                    ))
                } else {
                    Result.failure(IOException("HTTP ${response.code}"))
                }
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

@Serializable
data class ExchangeRate(
    val from: String,
    val to: String,
    val rate: Double,
    val timestamp: Long,
    val source: String
)

@Serializable
data class CoinGeckoResponse(
    val ippan: Map<String, Double>? = null
)

@Serializable
data class CoinMarketCapResponse(
    val data: Map<String, CoinMarketCapToken>? = null
)

@Serializable
data class CoinMarketCapToken(
    val quote: Map<String, CoinMarketCapQuote>? = null
)

@Serializable
data class CoinMarketCapQuote(
    val price: Double
)

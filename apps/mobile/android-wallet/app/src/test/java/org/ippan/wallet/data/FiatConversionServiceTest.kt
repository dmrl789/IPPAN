package org.ippan.wallet.data

import kotlinx.coroutines.test.runTest
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
class FiatConversionServiceTest {

    private val fiatConversionService = FiatConversionService()

    @Test
    fun `getExchangeRate returns valid exchange rate`() = runTest {
        // When
        val result = fiatConversionService.getExchangeRate("IPP", "USD")

        // Then
        assertTrue(result.isSuccess)
        val exchangeRate = result.getOrThrow()
        assertEquals("IPP", exchangeRate.from)
        assertEquals("USD", exchangeRate.to)
        assertTrue(exchangeRate.rate > 0)
        assertNotNull(exchangeRate.source)
    }

    @Test
    fun `getExchangeRate handles fallback gracefully`() = runTest {
        // When
        val result = fiatConversionService.getExchangeRate("UNKNOWN", "USD")

        // Then
        assertTrue(result.isSuccess)
        val exchangeRate = result.getOrThrow()
        assertEquals("UNKNOWN", exchangeRate.from)
        assertEquals("USD", exchangeRate.to)
        assertEquals(0.1, exchangeRate.rate) // Fallback rate
        assertEquals("fallback", exchangeRate.source)
    }
}

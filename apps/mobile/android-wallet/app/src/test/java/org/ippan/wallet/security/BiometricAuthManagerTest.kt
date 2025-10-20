package org.ippan.wallet.security

import android.content.Context
import androidx.biometric.BiometricManager
import androidx.test.core.app.ApplicationProvider
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.mockito.Mock
import org.mockito.MockitoAnnotations
import org.mockito.kotlin.whenever
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28])
class BiometricAuthManagerTest {

    @Mock
    private lateinit var mockContext: Context

    @Mock
    private lateinit var mockBiometricManager: BiometricManager

    private lateinit var biometricAuthManager: BiometricAuthManager

    @Before
    fun setUp() {
        MockitoAnnotations.openMocks(this)
        biometricAuthManager = BiometricAuthManager(mockContext)
    }

    @Test
    fun `isBiometricAvailable returns true when biometric is available`() {
        // Given
        whenever(mockBiometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_WEAK))
            .thenReturn(BiometricManager.BIOMETRIC_SUCCESS)

        // When
        val result = biometricAuthManager.isBiometricAvailable()

        // Then
        assertTrue(result)
    }

    @Test
    fun `isBiometricAvailable returns false when biometric is not available`() {
        // Given
        whenever(mockBiometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_WEAK))
            .thenReturn(BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE)

        // When
        val result = biometricAuthManager.isBiometricAvailable()

        // Then
        assertFalse(result)
    }

    @Test
    fun `isBiometricAvailable returns false when biometric is not enrolled`() {
        // Given
        whenever(mockBiometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_WEAK))
            .thenReturn(BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED)

        // When
        val result = biometricAuthManager.isBiometricAvailable()

        // Then
        assertFalse(result)
    }
}

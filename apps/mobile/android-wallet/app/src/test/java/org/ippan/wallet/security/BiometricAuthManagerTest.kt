package org.ippan.wallet.security

import android.content.Context
import androidx.biometric.BiometricManager
import androidx.test.core.app.ApplicationProvider
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import kotlin.test.assertNotNull

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28])
class BiometricAuthManagerTest {

    private lateinit var context: Context
    private lateinit var biometricAuthManager: BiometricAuthManager

    @Before
    fun setUp() {
        context = ApplicationProvider.getApplicationContext()
        biometricAuthManager = BiometricAuthManager(context)
    }

    @Test
    fun `isBiometricAvailable returns a valid result`() {
        // BiometricManager requires actual hardware/emulator to test properly
        // In unit tests, we just verify it doesn't crash
        val result = biometricAuthManager.isBiometricAvailable()
        assertNotNull(result)
    }

    @Test
    fun `BiometricAuthManager can be instantiated`() {
        assertNotNull(biometricAuthManager)
    }

    @Test
    fun `context is properly set`() {
        assertNotNull(context)
    }
}

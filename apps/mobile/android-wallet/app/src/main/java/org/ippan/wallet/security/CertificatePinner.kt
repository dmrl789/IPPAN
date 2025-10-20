package org.ippan.wallet.security

import okhttp3.CertificatePinner
import okhttp3.OkHttpClient
import java.security.cert.Certificate
import java.security.cert.X509Certificate

/**
 * Certificate pinning configuration for IPPAN API endpoints
 */
object CertificatePinner {
    
    /**
     * Configure certificate pinning for IPPAN nodes
     */
    fun createPinner(): CertificatePinner {
        return CertificatePinner.Builder()
            // Add pins for known IPPAN API endpoints
            .add("api.ippan.net", "sha256/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
            .add("api.ippan.org", "sha256/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
            .add("gateway.ippan.net", "sha256/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
            .add("gateway.ippan.org", "sha256/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
            // Add more pins as needed for production endpoints
            .build()
    }
    
    /**
     * Create an OkHttpClient with certificate pinning enabled
     */
    fun createSecureClient(
        baseUrls: List<String>,
        timeoutSeconds: Long = 30
    ): OkHttpClient {
        return OkHttpClient.Builder()
            .certificatePinner(createPinner())
            .connectTimeout(timeoutSeconds, java.util.concurrent.TimeUnit.SECONDS)
            .readTimeout(timeoutSeconds, java.util.concurrent.TimeUnit.SECONDS)
            .writeTimeout(timeoutSeconds, java.util.concurrent.TimeUnit.SECONDS)
            .build()
    }
    
    /**
     * Validate certificate chain for additional security
     */
    fun validateCertificateChain(certificates: List<Certificate>): Boolean {
        return try {
            certificates.forEach { cert ->
                if (cert is X509Certificate) {
                    // Check certificate validity
                    cert.checkValidity()
                    
                    // Additional validation can be added here
                    // e.g., check issuer, subject, extensions, etc.
                }
            }
            true
        } catch (e: Exception) {
            false
        }
    }
}

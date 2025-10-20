// Release configuration for IPPAN Android Wallet
android {
    buildTypes {
        getByName("release") {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            
            // Signing configuration
            signingConfig = signingConfigs.getByName("release")
            
            // Version information
            versionNameSuffix = ""
            
            // Build config fields
            buildConfigField("boolean", "DEBUG_MODE", "false")
            buildConfigField("String", "API_BASE_URL", "\"https://api.ippan.net\"")
            buildConfigField("boolean", "ENABLE_ANALYTICS", "true")
            buildConfigField("boolean", "ENABLE_CRASH_REPORTING", "true")
        }
        
        getByName("debug") {
            isMinifyEnabled = false
            isShrinkResources = false
            applicationIdSuffix = ".debug"
            versionNameSuffix = "-dev"
            
            buildConfigField("boolean", "DEBUG_MODE", "true")
            buildConfigField("String", "API_BASE_URL", "\"https://api-dev.ippan.net\"")
            buildConfigField("boolean", "ENABLE_ANALYTICS", "false")
            buildConfigField("boolean", "ENABLE_CRASH_REPORTING", "false")
        }
    }
    
    signingConfigs {
        create("release") {
            // These should be set via environment variables or keystore files
            keyAlias = System.getenv("KEY_ALIAS") ?: "ippan-release"
            keyPassword = System.getenv("KEY_PASSWORD") ?: ""
            storeFile = file(System.getenv("KEYSTORE_PATH") ?: "keystore/release.keystore")
            storePassword = System.getenv("KEYSTORE_PASSWORD") ?: ""
        }
    }
}

// Release tasks
tasks.register("prepareRelease") {
    group = "release"
    description = "Prepare release build"
    dependsOn("clean", "lint", "test", "dependencyCheckAnalyze")
}

tasks.register("buildRelease") {
    group = "release"
    description = "Build release APK and AAB"
    dependsOn("prepareRelease", "assembleRelease", "bundleRelease")
}

tasks.register("publishRelease") {
    group = "release"
    description = "Publish release to Play Store"
    dependsOn("buildRelease", "publishReleaseBundle")
}

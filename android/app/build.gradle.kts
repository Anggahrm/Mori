plugins {
    id("com.android.application")
}

android {
    namespace = "com.cendy.mori"
    compileSdk = 34
    
    defaultConfig {
        applicationId = "com.cendy.mori"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"
        
        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "armeabi-v7a"))
        }
    }
    
    buildTypes {
        release {
            isMinifyEnabled = false
        }
        debug {
            isDebuggable = true
        }
    }
    
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    
    buildFeatures {
        buildConfig = true
    }
    
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.6.1")
}

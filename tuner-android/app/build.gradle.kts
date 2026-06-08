import java.io.File

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jlleitschuh.gradle.ktlint")
}

android {
    namespace = "com.opentuner"
    compileSdk = 34
    defaultConfig {
        applicationId = "com.opentuner"
        minSdk = 26 // Oboe / AAudio
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86") }
    }
    buildTypes {
        debug { applicationIdSuffix = ".debug" }
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    packaging { resources.excludes += setOf("/META-INF/{AL2.0,LGPL2.1}") }
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
            java.srcDirs("src/main/kotlin")
        }
        getByName("test") { java.srcDirs("src/test/kotlin") }
        getByName("androidTest") { java.srcDirs("src/androidTest/kotlin") }
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.09.02")
    implementation(composeBom)
    androidTestImplementation(composeBom)
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.activity:activity-compose:1.9.2")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.6")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.6")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("com.google.oboe:oboe:1.9.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}

// ---- Cross-compile the Rust core and bundle the .so files ----
//
// For each Android ABI, builds tuner-core with the `jni` feature for the
// matching target triple, then copies libtuner_core.so into jniLibs/<abi>/.
// Requires a Rust toolchain with the Android targets installed and the NDK
// (ANDROID_NDK_HOME or NDK_HOME).

val abiToTriple =
    mapOf(
        "arm64-v8a" to "aarch64-linux-android",
        "armeabi-v7a" to "armv7-linux-androideabi",
        "x86_64" to "x86_64-linux-android",
        "x86" to "i686-linux-android",
    )

val abiToLinker =
    mapOf(
        "arm64-v8a" to "aarch64-linux-android26-clang",
        "armeabi-v7a" to "armv7a-linux-androideabi26-clang",
        "x86_64" to "x86_64-linux-android26-clang",
        "x86" to "i686-linux-android26-clang",
    )

tasks.register("buildRustCore") {
    group = "build"
    description = "Cross-compiles tuner-core (JNI) for every Android ABI."
    doLast {
        val coreDir = rootProject.projectDir.parentFile.resolve("tuner-core")
        val ndkHome =
            System.getenv("ANDROID_NDK_HOME")
                ?: System.getenv("NDK_HOME")
                ?: throw GradleException("Set ANDROID_NDK_HOME (or NDK_HOME) to build the Rust core.")
        val hostTag = "linux-x86_64" // adjust for macOS/Windows hosts
        val toolchainBin = File(ndkHome, "toolchains/llvm/prebuilt/$hostTag/bin")

        abiToTriple.forEach { (abi, triple) ->
            val linker = File(toolchainBin, abiToLinker.getValue(abi)).absolutePath
            val envVar = "CARGO_TARGET_${triple.uppercase().replace('-', '_')}_LINKER"

            exec {
                workingDir = coreDir
                environment(envVar, linker)
                environment("CC_$triple", linker)
                commandLine(
                    "cargo",
                    "build",
                    "--release",
                    "--features",
                    "jni",
                    "--target",
                    triple,
                )
            }

            val soSrc = coreDir.resolve("target/$triple/release/libtuner_core.so")
            val soDestDir = projectDir.resolve("src/main/jniLibs/$abi")
            soDestDir.mkdirs()
            soSrc.copyTo(soDestDir.resolve("libtuner_core.so"), overwrite = true)
        }
    }
}

tasks.named("preBuild") {
    dependsOn("buildRustCore")
}

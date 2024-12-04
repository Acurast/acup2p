import org.gradle.kotlin.dsl.support.listFilesOrdered

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.maven.publish)
    alias(libs.plugins.rust.gradle)
}

object Library {
    const val groupId = "com.acurast.p2p"
    const val artifactId = "acup2p"
    const val version = "0.0.1"
}

android {
    namespace = Library.groupId
    compileSdk = 35
    ndkVersion = sdkDirectory.resolve("ndk").listFilesOrdered().last().name

    defaultConfig {
        minSdk = 26
        version = Library.version

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    kotlinOptions {
        jvmTarget = "11"
    }
}

kotlin {
    explicitApiWarning()
}

cargo {
    module = "../../rust"
    libname = "acup2p"
    targets = listOf("arm", "arm64")
    profile = "release"
    prebuiltToolchains = true
}

publishing {
    publications {
        register<MavenPublication>("maven") {
            groupId = Library.groupId
            artifactId = Library.artifactId
            version = Library.version

            afterEvaluate {
                from(components["release"])
            }
        }
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    compileOnly(libs.jna)
    implementation(libs.kotlinx.coroutines.core)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
}

val ffiBuild: TaskProvider<Task> = tasks.register("ffiBuild", Task::class.java) {
    dependsOn("cargoBuild")

    doLast {
        exec {
            workingDir("../../rust")
            executable("cargo")
            args(
                "run", "--release",
                "--bin", "uniffi-bindgen",
                "generate",
                "--library", "target/aarch64-linux-android/release/libacup2p.so",
                "--language", "kotlin",
                "--out-dir", "../android/app/src/main/java/com/acurast/p2p")
        }
    }
}
plugins {
    kotlin("jvm") version "2.0.21"
}

repositories { mavenCentral() }

dependencies {
    // UniFFI's Kotlin bindings call the library through JNA.
    implementation("net.java.dev.jna:jna:5.17.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.12.2")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}



sourceSets {
    main { kotlin.srcDir("../target/bindings/kotlin") }
}

tasks.test {
    useJUnitPlatform()
    // The host library built by scripts/build-kotlin.sh (cargo build --profile mobile-release).
    val libDir = file("../target/mobile-release").absolutePath
    systemProperty("jna.library.path", libDir)
    systemProperty("lamina.fixtures", file("../../../fixtures").absolutePath)
    testLogging { events("passed", "failed", "skipped"); showStandardStreams = true; exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL }
}

// A minimal AAR without the Android Gradle Plugin (the SDK isn't needed to package one): the
// compiled bindings and idiomatic layer as classes.jar, the manifest, and jni/<abi>/*.so from
// scripts/build-android.sh. JNA comes from Maven as its own AAR (net.java.dev.jna:jna@aar).
// The product build (L6b) uses AGP; this measures what the AAR carries.
val aar by tasks.registering(Zip::class) {
    archiveFileName.set("lamina-ffi.aar")
    destinationDirectory.set(file("../target"))
    from(tasks.named("jar")) { rename { "classes.jar" } }
    from("src/aar/AndroidManifest.xml")
    from("../target/jniLibs") {
        into("jni")
        exclude("armeabi-v7a/**") // measured, not shipped (README)
    }
}

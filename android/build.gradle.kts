buildscript {
    repositories {
        gradlePluginPortal()
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:8.2.0")
    }
}

tasks.register("clean", Delete::class) {
    delete(rootProject.buildDir)
}

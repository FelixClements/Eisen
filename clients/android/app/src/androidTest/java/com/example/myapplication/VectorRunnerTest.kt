package com.example.myapplication

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

@RunWith(AndroidJUnit4::class)
class VectorRunnerTest {
    @Test
    fun runVectors() {
        val appContext = InstrumentationRegistry.getInstrumentation().targetContext
        appContext.assets.open("vectors.json").use { input ->
            File(appContext.filesDir, "vectors.json").outputStream().use { out ->
                input.copyTo(out)
            }
        }

        val jsonPath = File(appContext.filesDir, "vectors.json").absolutePath
        val outFile = File(appContext.filesDir, "vector-report-android.json")
        val exit = VectorRunner.runVectors(jsonPath, outFile.absolutePath)

        val report = outFile.readText()
        assertEquals("Vector runner returned non-zero exit", 0, exit)

        val root = JSONObject(report)
        assertEquals("Some vectors failed", 0, root.getInt("failed"))
    }
}

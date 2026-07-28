package com.example.myapplication

object VectorRunner {
    init {
        System.loadLibrary("eisen_core")
    }

    @JvmStatic
    external fun runVectors(jsonPath: String, outPath: String): Int
}

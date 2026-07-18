package com.example.myapplication.data.local

import android.content.Context
import androidx.room.Room

object DatabaseProvider {
    @Volatile
    private var instance: AppDatabase? = null

    fun getDatabase(context: Context): AppDatabase = instance ?: synchronized(this) {
        instance ?: Room.databaseBuilder(
            context.applicationContext,
            AppDatabase::class.java,
            DATABASE_NAME,
        ).addMigrations(AppDatabase.MIGRATION_1_2)
            .build()
            .also { instance = it }
    }

    private const val DATABASE_NAME = "eisenhower_tasks.db"
}

package com.example.myapplication.data.local

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "tasks")
data class TaskEntity(
    @PrimaryKey(autoGenerate = true)
    val id: Long = 0,
    val title: String,
    val description: String?,
    val isImportant: Boolean,
    val isUrgent: Boolean,
    val dueDate: Long?,
    val reminderAt: Long? = null,
    val isCompleted: Boolean,
    val isArchived: Boolean,
    val isPinned: Boolean,
    val category: String?,
    val createdAt: Long,
    val updatedAt: Long,
)

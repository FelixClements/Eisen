package com.example.myapplication.data.local

import com.example.myapplication.domain.Task

fun TaskEntity.toDomain(): Task = Task(
    id = id,
    title = title,
    description = description,
    isImportant = isImportant,
    isUrgent = isUrgent,
    dueDate = dueDate,
    reminderAt = reminderAt,
    isCompleted = isCompleted,
    isArchived = isArchived,
    isPinned = isPinned,
    category = category,
    createdAt = createdAt,
    updatedAt = updatedAt,
)

fun Task.toEntity(): TaskEntity = TaskEntity(
    id = id,
    title = title,
    description = description,
    isImportant = isImportant,
    isUrgent = isUrgent,
    dueDate = dueDate,
    reminderAt = reminderAt,
    isCompleted = isCompleted,
    isArchived = isArchived,
    isPinned = isPinned,
    category = category,
    createdAt = createdAt,
    updatedAt = updatedAt,
)

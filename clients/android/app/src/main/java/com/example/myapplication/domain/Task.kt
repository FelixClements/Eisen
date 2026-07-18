package com.example.myapplication.domain

data class Task(
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
) {
    val eisenhowerCategory: EisenhowerCategory
        get() = EisenhowerCategory.from(
            isImportant = isImportant,
            isUrgent = isUrgent,
        )
}

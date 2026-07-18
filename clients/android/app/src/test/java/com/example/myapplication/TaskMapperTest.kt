package com.example.myapplication

import com.example.myapplication.data.local.TaskEntity
import com.example.myapplication.data.local.toDomain
import com.example.myapplication.data.local.toEntity
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import org.junit.Assert.assertEquals
import org.junit.Test

class TaskMapperTest {
    @Test
    fun `entity maps to domain with important fields`() {
        val entity = TaskEntity(
            id = 7L,
            title = "Pay electricity bill",
            description = "Before 16:00",
            isImportant = true,
            isUrgent = true,
            dueDate = 1_700_000_000_000L,
            reminderAt = 1_700_000_600_000L,
            isCompleted = false,
            isArchived = false,
            isPinned = true,
            category = "Home",
            createdAt = 1_699_999_000_000L,
            updatedAt = 1_699_999_500_000L,
        )

        val task = entity.toDomain()

        assertEquals(entity.id, task.id)
        assertEquals(entity.title, task.title)
        assertEquals(entity.description, task.description)
        assertEquals(entity.isImportant, task.isImportant)
        assertEquals(entity.isUrgent, task.isUrgent)
        assertEquals(entity.dueDate, task.dueDate)
        assertEquals(entity.reminderAt, task.reminderAt)
        assertEquals(entity.isCompleted, task.isCompleted)
        assertEquals(entity.isArchived, task.isArchived)
        assertEquals(entity.isPinned, task.isPinned)
        assertEquals(entity.category, task.category)
        assertEquals(EisenhowerCategory.DO_NOW, task.eisenhowerCategory)
    }

    @Test
    fun `domain maps to entity and back`() {
        val task = Task(
            id = 11L,
            title = "Study Kotlin Room",
            description = null,
            isImportant = true,
            isUrgent = false,
            dueDate = null,
            reminderAt = 1_700_003_000_000L,
            isCompleted = true,
            isArchived = false,
            isPinned = false,
            category = "Learning",
            createdAt = 1_700_001_000_000L,
            updatedAt = 1_700_002_000_000L,
        )

        val roundTrip = task.toEntity().toDomain()

        assertEquals(task, roundTrip)
        assertEquals(EisenhowerCategory.SCHEDULE, roundTrip.eisenhowerCategory)
    }
}

package com.example.myapplication.data

import com.example.myapplication.data.local.TaskDao
import com.example.myapplication.data.local.TaskEntity
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LocalTaskRepositoryTest {
    @Test
    fun `unarchive preserves task fields and only updates archive state and updated timestamp`() = runTest {
        val original = TaskEntity(
            id = 42L,
            title = "Prepare report",
            description = "Include meeting notes",
            isImportant = true,
            isUrgent = false,
            dueDate = 1_700_000_000_000L,
            reminderAt = 1_700_000_600_000L,
            isCompleted = true,
            isArchived = true,
            isPinned = true,
            category = "Work",
            createdAt = 1_699_999_000_000L,
            updatedAt = 1_699_999_500_000L,
        )
        val newUpdatedAt = 1_700_001_000_000L
        val dao = FakeTaskDao(original)
        val repository = LocalTaskRepository(dao) { newUpdatedAt }

        repository.unarchiveTask(original.id)

        assertEquals(0, dao.getTaskByIdCalls)
        val task = repository.getTaskById(original.id).first()!!
        assertFalse(task.isArchived)
        assertTrue(task.isCompleted)
        assertEquals(original.reminderAt, task.reminderAt)
        assertEquals(original.title, task.title)
        assertEquals(original.description, task.description)
        assertEquals(original.isImportant, task.isImportant)
        assertEquals(original.isUrgent, task.isUrgent)
        assertEquals(original.category, task.category)
        assertEquals(original.dueDate, task.dueDate)
        assertEquals(original.isPinned, task.isPinned)
        assertEquals(original.createdAt, task.createdAt)
        assertEquals(newUpdatedAt, task.updatedAt)
        assertEquals(UnarchiveCall(original.id, newUpdatedAt), dao.unarchiveCall)
        assertEquals(0, dao.updateTaskCalls)
    }
}

private data class UnarchiveCall(
    val id: Long,
    val updatedAt: Long,
)

private class FakeTaskDao(initialEntity: TaskEntity) : TaskDao {
    private val taskFlow = MutableStateFlow<TaskEntity?>(initialEntity)
    var unarchiveCall: UnarchiveCall? = null
    var getTaskByIdCalls = 0
    var updateTaskCalls = 0

    override fun observeActiveTasks(): Flow<List<TaskEntity>> = flowOf(emptyList())

    override fun observeCompletedTasks(): Flow<List<TaskEntity>> = flowOf(emptyList())

    override fun observeArchivedTasks(): Flow<List<TaskEntity>> = flowOf(emptyList())

    override fun observeAllTasks(): Flow<List<TaskEntity>> = flowOf(taskFlow.value?.let { listOf(it) } ?: emptyList())

    override fun searchTasks(query: String): Flow<List<TaskEntity>> = flowOf(emptyList())

    override fun getTaskById(id: Long): Flow<TaskEntity?> {
        getTaskByIdCalls++
        return taskFlow
    }

    override suspend fun getTaskByIdNow(id: Long): TaskEntity? = taskFlow.value

    override suspend fun insertTask(entity: TaskEntity): Long = entity.id

    override suspend fun updateTask(entity: TaskEntity) {
        updateTaskCalls++
        taskFlow.value = entity
    }

    override suspend fun unarchiveTask(id: Long, updatedAt: Long) {
        unarchiveCall = UnarchiveCall(id, updatedAt)
        taskFlow.value = taskFlow.value?.copy(isArchived = false, updatedAt = updatedAt)
    }

    override suspend fun deleteTask(entity: TaskEntity) = Unit

    override suspend fun deleteTaskById(id: Long) = Unit
}

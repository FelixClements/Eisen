package com.example.myapplication.ui.task

import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskRepository
import com.example.myapplication.ui.home.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class TaskDetailViewModelTest {
    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    @Test
    fun `updateTask forwards to repository`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = RecordingTaskRepository()
        val viewModel = TaskDetailViewModel(taskId = 1L, repository = repository)

        viewModel.updateTask(sampleTask(id = 1L, title = "Updated"))
        advanceUntilIdle()

        assertEquals("Updated", repository.updatedTasks.single().title)
    }

    @Test
    fun `updateTask emits OperationFailed when repository fails`() = runTest(mainDispatcherRule.testDispatcher) {
        val viewModel = TaskDetailViewModel(taskId = 1L, repository = FailingTaskRepository())
        val events = mutableListOf<TaskDetailUiEvent>()
        val job = launch { viewModel.events.toList(events) }
        advanceUntilIdle()

        viewModel.updateTask(sampleTask(id = 1L, title = "Updated"))
        advanceUntilIdle()

        assertEquals(1, events.size)
        assertTrue(events.single() is TaskDetailUiEvent.OperationFailed)
        job.cancel()
    }
}

private fun sampleTask(id: Long, title: String): Task = Task(
    id = id,
    title = title,
    description = null,
    isImportant = true,
    isUrgent = true,
    dueDate = null,
    reminderAt = null,
    isCompleted = false,
    isArchived = false,
    isPinned = false,
    category = null,
    createdAt = 0L,
    updatedAt = 0L,
)

private open class RecordingTaskRepository : TaskRepository {
    val updatedTasks = mutableListOf<Task>()

    override fun observeActiveTasks(): Flow<List<Task>> = flowOf(emptyList())
    override fun observeCompletedTasks(): Flow<List<Task>> = flowOf(emptyList())
    override fun observeArchivedTasks(): Flow<List<Task>> = flowOf(emptyList())
    override fun observeAllTasks(): Flow<List<Task>> = flowOf(emptyList())
    override fun searchTasks(query: String): Flow<List<Task>> = flowOf(emptyList())
    override fun getTaskById(id: Long): Flow<Task?> = flowOf(null)
    override suspend fun addTask(task: Task): Long = 0L
    override suspend fun updateTask(task: Task) {
        updatedTasks += task
    }
    override suspend fun completeTask(id: Long, isCompleted: Boolean) = Unit
    override suspend fun archiveTask(id: Long) = Unit
    override suspend fun unarchiveTask(id: Long) = Unit
    override suspend fun deleteTask(id: Long) = Unit
    override suspend fun moveTask(id: Long, category: EisenhowerCategory) = Unit
}

private class FailingTaskRepository : RecordingTaskRepository() {
    override suspend fun updateTask(task: Task) = throw RuntimeException("fail")
}

package com.example.myapplication.ui.history

import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskReminderScheduler
import com.example.myapplication.domain.TaskRepository
import com.example.myapplication.ui.home.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class HistoryViewModelTest {
    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    @Test
    fun `completed tab shows non-archived completed tasks`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeHistoryRepository(
            completedTasks = listOf(taskWith(id = 1L, isCompleted = true)),
        )
        val viewModel = HistoryViewModel(repository, NoOpReminderScheduler)
        advanceUntilIdle()

        assertEquals(1, viewModel.uiState.value.completedTasks.size)
        assertEquals(1L, viewModel.uiState.value.completedTasks[0].id)
        assertTrue(viewModel.uiState.value.archivedTasks.isEmpty())
    }

    @Test
    fun `archive tab shows all archived tasks`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeHistoryRepository(
            archivedTasks = listOf(
                taskWith(id = 2L, isArchived = true, isCompleted = false),
                taskWith(id = 3L, isArchived = true, isCompleted = true),
            ),
        )
        val viewModel = HistoryViewModel(repository, NoOpReminderScheduler)
        advanceUntilIdle()

        assertEquals(2, viewModel.uiState.value.archivedTasks.size)
        assertTrue(viewModel.uiState.value.completedTasks.isEmpty())
    }

    @Test
    fun `uncompleteTask moves task from completed to active`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeHistoryRepository(
            completedTasks = listOf(taskWith(id = 4L, isCompleted = true)),
        )
        val viewModel = HistoryViewModel(repository, NoOpReminderScheduler)
        advanceUntilIdle()

        viewModel.uncompleteTask(4L)
        advanceUntilIdle()

        assertTrue(viewModel.uiState.value.completedTasks.isEmpty())
    }

    @Test
    fun `unarchiveTask removes task from archive`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeHistoryRepository(
            archivedTasks = listOf(taskWith(id = 5L, isArchived = true)),
        )
        val viewModel = HistoryViewModel(repository, NoOpReminderScheduler)
        advanceUntilIdle()

        viewModel.unarchiveTask(5L)
        advanceUntilIdle()

        assertTrue(viewModel.uiState.value.archivedTasks.isEmpty())
    }
}

private fun taskWith(
    id: Long,
    isCompleted: Boolean = false,
    isArchived: Boolean = false,
): Task = Task(
    id = id,
    title = "Task $id",
    description = null,
    isImportant = true,
    isUrgent = true,
    dueDate = null,
    reminderAt = null,
    isCompleted = isCompleted,
    isArchived = isArchived,
    isPinned = false,
    category = null,
    createdAt = 0L,
    updatedAt = 0L,
)

private object NoOpReminderScheduler : TaskReminderScheduler {
    override fun schedule(taskId: Long, reminderAt: Long): Result<Unit> = Result.success(Unit)
    override fun cancel(taskId: Long): Result<Unit> = Result.success(Unit)
    override fun getScheduledTaskIds(): Flow<Set<Long>> = flowOf(emptySet())
}

private class FakeHistoryRepository(
    completedTasks: List<Task> = emptyList(),
    archivedTasks: List<Task> = emptyList(),
) : TaskRepository {
    private val tasksById = mutableMapOf<Long, Task>()
    private val completedFlow = MutableStateFlow(completedTasks)
    private val archivedFlow = MutableStateFlow(archivedTasks)

    init {
        completedTasks.forEach { tasksById[it.id] = it }
        archivedTasks.forEach { tasksById[it.id] = it }
    }

    override fun observeActiveTasks(): Flow<List<Task>> = flowOf(emptyList())
    override fun observeCompletedTasks(): Flow<List<Task>> = completedFlow
    override fun observeArchivedTasks(): Flow<List<Task>> = archivedFlow
    override fun observeAllTasks(): Flow<List<Task>> = flowOf(tasksById.values.toList())
    override fun searchTasks(query: String): Flow<List<Task>> = flowOf(emptyList())

    override fun getTaskById(id: Long): Flow<Task?> = flowOf(tasksById[id])

    override suspend fun addTask(task: Task): Long = 0L
    override suspend fun updateTask(task: Task) = Unit

    override suspend fun completeTask(id: Long, isCompleted: Boolean) {
        tasksById[id]?.let { task ->
            tasksById[id] = task.copy(isCompleted = isCompleted)
            completedFlow.value = tasksById.values.filter { it.isCompleted && !it.isArchived }
        }
    }

    override suspend fun archiveTask(id: Long) {
        tasksById[id]?.let { task ->
            tasksById[id] = task.copy(isArchived = true)
            archivedFlow.value = tasksById.values.filter { it.isArchived }
            completedFlow.value = tasksById.values.filter { it.isCompleted && !it.isArchived }
        }
    }

    override suspend fun unarchiveTask(id: Long) {
        tasksById[id]?.let { task ->
            tasksById[id] = task.copy(isArchived = false)
            archivedFlow.value = tasksById.values.filter { it.isArchived }
            completedFlow.value = tasksById.values.filter { it.isCompleted && !it.isArchived }
        }
    }

    override suspend fun deleteTask(id: Long) = Unit
    override suspend fun moveTask(id: Long, category: EisenhowerCategory) = Unit
}

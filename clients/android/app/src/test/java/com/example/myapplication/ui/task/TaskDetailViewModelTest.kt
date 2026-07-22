package com.example.myapplication.ui.task

import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TestWatcher
import org.junit.runner.Description

@OptIn(ExperimentalCoroutinesApi::class)
class TaskDetailViewModelTest {
    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    @Test
    fun `task exposes the repository entry for the requested id`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeTaskRepository(initialTasks = listOf(sampleTask(id = 42L, title = "Renew passport")))
        val viewModel = TaskDetailViewModel(taskId = 42L, repository = repository)

        backgroundScope.launchCollecting(viewModel)
        advanceUntilIdle()

        assertEquals("Renew passport", viewModel.task.value?.title)
        assertEquals(42L, viewModel.task.value?.id)
    }

    @Test
    fun `task is null before collection begins`() {
        val repository = FakeTaskRepository(initialTasks = listOf(sampleTask(id = 42L, title = "Renew passport")))
        val viewModel = TaskDetailViewModel(taskId = 42L, repository = repository)

        assertNull(viewModel.task.value)
    }

    @Test
    fun `task is null for an unknown id`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeTaskRepository(initialTasks = listOf(sampleTask(id = 1L, title = "Known")))
        val viewModel = TaskDetailViewModel(taskId = 999L, repository = repository)

        backgroundScope.launchCollecting(viewModel)
        advanceUntilIdle()

        assertNull(viewModel.task.value)
    }

    @Test
    fun `task reflects repository updates while collected`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeTaskRepository(initialTasks = listOf(sampleTask(id = 7L, title = "Draft")))
        val viewModel = TaskDetailViewModel(taskId = 7L, repository = repository)

        backgroundScope.launchCollecting(viewModel)
        advanceUntilIdle()
        repository.updateTask(sampleTask(id = 7L, title = "Final"))
        advanceUntilIdle()

        assertEquals("Final", viewModel.task.value?.title)
    }

    @Test
    fun `updateTask forwards the task to the repository`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeTaskRepository(initialTasks = listOf(sampleTask(id = 3L, title = "Old")))
        val viewModel = TaskDetailViewModel(taskId = 3L, repository = repository)

        val edited = sampleTask(id = 3L, title = "New title")
        viewModel.updateTask(edited)
        advanceUntilIdle()

        assertEquals(listOf(edited), repository.updatedTasks)
    }

    @Test
    fun `factory builds a view model bound to the given id`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeTaskRepository(initialTasks = listOf(sampleTask(id = 5L, title = "Factory task")))

        val viewModel = TaskDetailViewModel.Factory(taskId = 5L, repository = repository)
            .create(TaskDetailViewModel::class.java)

        backgroundScope.launchCollecting(viewModel)
        advanceUntilIdle()

        assertEquals("Factory task", viewModel.task.value?.title)
    }
}

private fun kotlinx.coroutines.CoroutineScope.launchCollecting(viewModel: TaskDetailViewModel) {
    launch { viewModel.task.collect {} }
}

private fun sampleTask(id: Long, title: String): Task = Task(
    id = id,
    title = title,
    description = null,
    isImportant = true,
    isUrgent = false,
    dueDate = null,
    reminderAt = null,
    isCompleted = false,
    isArchived = false,
    isPinned = false,
    category = null,
    createdAt = 0L,
    updatedAt = 0L,
)

@OptIn(ExperimentalCoroutinesApi::class)
class MainDispatcherRule(
    val testDispatcher: TestDispatcher = StandardTestDispatcher(),
) : TestWatcher() {
    override fun starting(description: Description) {
        Dispatchers.setMain(testDispatcher)
    }

    override fun finished(description: Description) {
        Dispatchers.resetMain()
    }
}

private class FakeTaskRepository(
    initialTasks: List<Task> = emptyList(),
) : TaskRepository {
    private val tasks = MutableStateFlow(initialTasks.associateBy { it.id })
    val updatedTasks = mutableListOf<Task>()

    override fun observeActiveTasks(): Flow<List<Task>> = flowOf(emptyList())

    override fun observeCompletedTasks(): Flow<List<Task>> = flowOf(emptyList())

    override fun observeArchivedTasks(): Flow<List<Task>> = flowOf(emptyList())

    override fun observeAllTasks(): Flow<List<Task>> = flowOf(emptyList())

    override fun searchTasks(query: String): Flow<List<Task>> = flowOf(emptyList())

    override fun getTaskById(id: Long): Flow<Task?> = tasks.map { it[id] }

    override suspend fun addTask(task: Task): Long = task.id

    override suspend fun updateTask(task: Task) {
        updatedTasks += task
        tasks.value = tasks.value + (task.id to task)
    }

    override suspend fun completeTask(id: Long, isCompleted: Boolean) = Unit

    override suspend fun archiveTask(id: Long) = Unit

    override suspend fun unarchiveTask(id: Long) = Unit

    override suspend fun deleteTask(id: Long) = Unit

    override suspend fun moveTask(id: Long, category: EisenhowerCategory) = Unit
}

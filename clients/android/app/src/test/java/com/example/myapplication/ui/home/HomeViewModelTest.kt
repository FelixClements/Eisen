package com.example.myapplication.ui.home

import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskReminderScheduler
import com.example.myapplication.domain.TaskRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TestWatcher
import org.junit.runner.Description

@OptIn(ExperimentalCoroutinesApi::class)
class HomeViewModelTest {
    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    @Test
    fun `addTask trims title and sets defaults from category`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeTaskRepository()
        val viewModel = HomeViewModel(
            repository = repository,
            clock = { 123L },
        )

        viewModel.addTask(
            title = "  Buy milk  ",
            category = EisenhowerCategory.SCHEDULE,
            description = "Groceries",
            dueDate = 456L,
        )
        advanceUntilIdle()

        val addedTask = repository.addedTasks.single()
        assertEquals("Buy milk", addedTask.title)
        assertEquals("Groceries", addedTask.description)
        assertEquals(456L, addedTask.dueDate)
        assertTrue(addedTask.isImportant)
        assertFalse(addedTask.isUrgent)
        assertFalse(addedTask.isCompleted)
        assertFalse(addedTask.isArchived)
        assertFalse(addedTask.isPinned)
        assertEquals(123L, addedTask.createdAt)
        assertEquals(123L, addedTask.updatedAt)
    }

    @Test
    fun `addTask ignores blank title`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeTaskRepository()
        val viewModel = HomeViewModel(repository)

        viewModel.addTask(" \n\t ", EisenhowerCategory.DO_NOW)
        advanceUntilIdle()

        assertTrue(repository.addedTasks.isEmpty())
    }

    @Test
    fun `addTask schedules a persisted reminder`() = runTest(mainDispatcherRule.testDispatcher) {
        val repository = FakeTaskRepository()
        val scheduler = RecordingReminderScheduler()
        val viewModel = HomeViewModel(repository, scheduler)

        viewModel.addTask(
            title = "Call the dentist",
            category = EisenhowerCategory.DO_NOW,
            reminderAt = 789L,
        )
        advanceUntilIdle()

        assertEquals(listOf(1L to 789L), scheduler.scheduledReminders)
    }

    @Test
    fun `completion and archive cancel reminders`() = runTest(mainDispatcherRule.testDispatcher) {
        val scheduler = RecordingReminderScheduler(initialScheduledTaskIds = setOf(4L, 8L))
        val repository = FakeTaskRepository(
            initialTasks = listOf(
                taskWithReminder(id = 4L, reminderAt = 200L),
                taskWithReminder(id = 8L, reminderAt = 200L),
            ),
        )
        val viewModel = HomeViewModel(repository, scheduler, clock = { 100L })

        viewModel.completeTask(task = repository.task(4L), isCompleted = true)
        viewModel.archiveTask(task = repository.task(8L))
        advanceUntilIdle()

        assertEquals(listOf(4L, 8L), scheduler.cancelledTaskIds)
    }

    @Test
    fun `uncompleting a task reschedules its future reminder`() = runTest(mainDispatcherRule.testDispatcher) {
        val scheduler = RecordingReminderScheduler()
        val repository = FakeTaskRepository(
            initialTasks = listOf(taskWithReminder(id = 4L, reminderAt = 200L, isCompleted = true)),
        )
        val viewModel = HomeViewModel(repository, scheduler, clock = { 100L })

        viewModel.completeTask(task = repository.task(4L), isCompleted = false)
        advanceUntilIdle()

        assertEquals(listOf(4L to 200L), scheduler.scheduledReminders)
    }

    @Test
    fun `initialization reconciles future active reminders`() = runTest(mainDispatcherRule.testDispatcher) {
        val scheduler = RecordingReminderScheduler()
        val repository = FakeTaskRepository(
            initialTasks = listOf(
                taskWithReminder(id = 9L, reminderAt = 200L),
                taskWithReminder(id = 10L, reminderAt = 100L),
            ),
        )

        HomeViewModel(repository, scheduler, clock = { 100L })
        advanceUntilIdle()

        assertEquals(listOf(9L to 200L), scheduler.scheduledReminders)
    }

    @Test
    fun `unarchiving an active task reschedules its future reminder`() = runTest(mainDispatcherRule.testDispatcher) {
        val scheduler = RecordingReminderScheduler()
        val repository = FakeTaskRepository(
            initialTasks = listOf(
                taskWithReminder(id = 5L, reminderAt = 200L).copy(isArchived = true)
            ),
        )
        val viewModel = HomeViewModel(repository, scheduler, clock = { 100L })

        viewModel.unarchiveTask(id = 5L)
        advanceUntilIdle()

        assertEquals(listOf(5L to 200L), scheduler.scheduledReminders)
    }

    @Test
    fun `unarchiving a completed task does not reschedule reminder`() = runTest(mainDispatcherRule.testDispatcher) {
        val scheduler = RecordingReminderScheduler()
        val repository = FakeTaskRepository(
            initialTasks = listOf(
                taskWithReminder(id = 6L, reminderAt = 200L).copy(isArchived = true, isCompleted = true)
            ),
        )
        val viewModel = HomeViewModel(repository, scheduler, clock = { 100L })

        viewModel.unarchiveTask(id = 6L)
        advanceUntilIdle()

        assertTrue(scheduler.scheduledReminders.isEmpty())
    }

    @Test
    fun `unarchiveTask emits OperationFailed when repository fails`() = runTest(mainDispatcherRule.testDispatcher) {
        val viewModel = HomeViewModel(FailingTaskRepository())
        val events = mutableListOf<HomeUiEvent>()
        val job = launch { viewModel.events.toList(events) }
        advanceUntilIdle()

        viewModel.unarchiveTask(id = 1L)
        advanceUntilIdle()

        assertEquals(1, events.size)
        assertTrue(events.single() is HomeUiEvent.OperationFailed)
        job.cancel()
    }

    @Test
    fun `moveTask emits OperationFailed when repository fails`() = runTest(mainDispatcherRule.testDispatcher) {
        val viewModel = HomeViewModel(FailingTaskRepository())
        val events = mutableListOf<HomeUiEvent>()
        val job = launch { viewModel.events.toList(events) }
        advanceUntilIdle()

        viewModel.moveTask(id = 1L, category = EisenhowerCategory.DO_NOW)
        advanceUntilIdle()

        assertEquals(1, events.size)
        assertTrue(events.single() is HomeUiEvent.OperationFailed)
        job.cancel()
    }
}

private class FailingTaskRepository : TaskRepository {
    override fun observeActiveTasks(): Flow<List<Task>> = flowOf(emptyList())
    override fun observeCompletedTasks(): Flow<List<Task>> = flowOf(emptyList())
    override fun observeArchivedTasks(): Flow<List<Task>> = flowOf(emptyList())
    override fun observeAllTasks(): Flow<List<Task>> = flowOf(emptyList())
    override fun searchTasks(query: String): Flow<List<Task>> = flowOf(emptyList())
    override fun getTaskById(id: Long): Flow<Task?> = flowOf(null)
    override suspend fun addTask(task: Task): Long = throw RuntimeRepositoryException()
    override suspend fun updateTask(task: Task) = throw RuntimeRepositoryException()
    override suspend fun completeTask(id: Long, isCompleted: Boolean) = throw RuntimeRepositoryException()
    override suspend fun archiveTask(id: Long) = throw RuntimeRepositoryException()
    override suspend fun unarchiveTask(id: Long) = throw RuntimeRepositoryException()
    override suspend fun deleteTask(id: Long) = throw RuntimeRepositoryException()
    override suspend fun moveTask(id: Long, category: EisenhowerCategory) = throw RuntimeRepositoryException()
}

private class RuntimeRepositoryException : RuntimeException("Simulated repository failure")

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
    private val tasksById = initialTasks.associateByTo(mutableMapOf()) { it.id }
    private val activeTasks = MutableStateFlow(initialTasks.filter { !it.isCompleted && !it.isArchived })
    private val allTasks = MutableStateFlow(initialTasks)
    val addedTasks = mutableListOf<Task>()

    fun task(id: Long): Task = tasksById.getValue(id)

    override fun observeActiveTasks(): Flow<List<Task>> = activeTasks

    override fun observeCompletedTasks(): Flow<List<Task>> = flowOf(emptyList())

    override fun observeArchivedTasks(): Flow<List<Task>> = flowOf(emptyList())

    override fun observeAllTasks(): Flow<List<Task>> = allTasks

    override fun searchTasks(query: String): Flow<List<Task>> = flowOf(emptyList())

    override fun getTaskById(id: Long): Flow<Task?> = flowOf(tasksById[id])

    override suspend fun addTask(task: Task): Long {
        addedTasks += task
        val id = addedTasks.size.toLong()
        tasksById[id] = task.copy(id = id)
        activeTasks.value = tasksById.values.filter { !it.isCompleted && !it.isArchived }
        allTasks.value = tasksById.values.toList()
        return id
    }

    override suspend fun updateTask(task: Task) = Unit
    override suspend fun completeTask(id: Long, isCompleted: Boolean) {
        tasksById[id]?.let { task ->
            tasksById[id] = task.copy(isCompleted = isCompleted)
            activeTasks.value = tasksById.values.filter { !it.isCompleted && !it.isArchived }
            allTasks.value = tasksById.values.toList()
        }
    }

    override suspend fun archiveTask(id: Long) {
        tasksById[id]?.let { task ->
            tasksById[id] = task.copy(isArchived = true)
            activeTasks.value = tasksById.values.filter { !it.isCompleted && !it.isArchived }
            allTasks.value = tasksById.values.toList()
        }
    }

    override suspend fun unarchiveTask(id: Long) {
        tasksById[id]?.let { task ->
            tasksById[id] = task.copy(isArchived = false)
            activeTasks.value = tasksById.values.filter { !it.isCompleted && !it.isArchived }
            allTasks.value = tasksById.values.toList()
        }
    }

    override suspend fun deleteTask(id: Long) = Unit
    override suspend fun moveTask(id: Long, category: EisenhowerCategory) = Unit
}

private fun taskWithReminder(
    id: Long,
    reminderAt: Long,
    isCompleted: Boolean = false,
): Task = Task(
    id = id,
    title = "Reminder task",
    description = null,
    isImportant = true,
    isUrgent = true,
    dueDate = null,
    reminderAt = reminderAt,
    isCompleted = isCompleted,
    isArchived = false,
    isPinned = false,
    category = null,
    createdAt = 0L,
    updatedAt = 0L,
)

private class RecordingReminderScheduler(
    initialScheduledTaskIds: Set<Long> = emptySet(),
) : TaskReminderScheduler {
    val scheduledReminders = mutableListOf<Pair<Long, Long>>()
    val cancelledTaskIds = mutableListOf<Long>()
    private val scheduledTaskIds = MutableStateFlow(initialScheduledTaskIds)

    override fun schedule(taskId: Long, reminderAt: Long): Result<Unit> {
        scheduledReminders += taskId to reminderAt
        scheduledTaskIds.value += taskId
        return Result.success(Unit)
    }

    override fun cancel(taskId: Long): Result<Unit> {
        cancelledTaskIds += taskId
        scheduledTaskIds.value -= taskId
        return Result.success(Unit)
    }

    override fun getScheduledTaskIds(): Flow<Set<Long>> = scheduledTaskIds
}

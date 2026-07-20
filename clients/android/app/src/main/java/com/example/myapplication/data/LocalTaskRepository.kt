package com.example.myapplication.data

import com.example.myapplication.data.local.TaskDao
import com.example.myapplication.data.local.toDomain
import com.example.myapplication.data.local.toEntity
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskRepository
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map

class LocalTaskRepository(
    private val taskDao: TaskDao,
    private val clock: () -> Long = { System.currentTimeMillis() },
) : TaskRepository {
    override fun observeActiveTasks(): Flow<List<Task>> =
        taskDao.observeActiveTasks().map { entities -> entities.map { it.toDomain() } }

    override fun observeCompletedTasks(): Flow<List<Task>> =
        taskDao.observeCompletedTasks().map { entities -> entities.map { it.toDomain() } }

    override fun observeArchivedTasks(): Flow<List<Task>> =
        taskDao.observeArchivedTasks().map { entities -> entities.map { it.toDomain() } }

    override fun observeAllTasks(): Flow<List<Task>> =
        taskDao.observeAllTasks().map { entities -> entities.map { it.toDomain() } }

    override fun searchTasks(query: String): Flow<List<Task>> =
        taskDao.searchTasks(query).map { entities -> entities.map { it.toDomain() } }

    override fun getTaskById(id: Long): Flow<Task?> =
        taskDao.getTaskById(id).map { entity -> entity?.toDomain() }

    override suspend fun addTask(task: Task): Long = taskDao.insertTask(task.toEntity())

    override suspend fun updateTask(task: Task) {
        taskDao.updateTask(task.copy(updatedAt = clock()).toEntity())
    }

    override suspend fun completeTask(id: Long, isCompleted: Boolean) {
        val entity = taskDao.getTaskById(id).first() ?: return
        taskDao.updateTask(
            entity.copy(
                isCompleted = isCompleted,
                updatedAt = clock(),
            )
        )
    }

    override suspend fun archiveTask(id: Long) {
        val entity = taskDao.getTaskById(id).first() ?: return
        taskDao.updateTask(
            entity.copy(
                isArchived = true,
                updatedAt = clock(),
            )
        )
    }

    override suspend fun unarchiveTask(id: Long) {
        taskDao.unarchiveTask(id, clock())
    }

    override suspend fun deleteTask(id: Long) {
        taskDao.deleteTaskById(id)
    }

    override suspend fun moveTask(id: Long, category: EisenhowerCategory) {
        val entity = taskDao.getTaskById(id).first() ?: return
        taskDao.updateTask(
            entity.copy(
                isImportant = category.isImportant,
                isUrgent = category.isUrgent,
                updatedAt = clock(),
            )
        )
    }
}

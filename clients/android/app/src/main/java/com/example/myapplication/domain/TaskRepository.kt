package com.example.myapplication.domain

import kotlinx.coroutines.flow.Flow

interface TaskRepository {
    fun observeActiveTasks(): Flow<List<Task>>

    fun observeCompletedTasks(): Flow<List<Task>>

    fun observeArchivedTasks(): Flow<List<Task>>

    /** Observes all tasks, regardless of their completed or archived state. */
    fun observeAllTasks(): Flow<List<Task>>

    fun searchTasks(query: String): Flow<List<Task>>

    fun getTaskById(id: Long): Flow<Task?>

    suspend fun addTask(task: Task): Long

    suspend fun updateTask(task: Task)

    suspend fun completeTask(id: Long, isCompleted: Boolean)

    suspend fun archiveTask(id: Long)

    suspend fun unarchiveTask(id: Long)

    suspend fun deleteTask(id: Long)

    suspend fun moveTask(id: Long, category: EisenhowerCategory)
}

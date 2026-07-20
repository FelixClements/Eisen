package com.example.myapplication.data.local

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import androidx.room.Update
import kotlinx.coroutines.flow.Flow

@Dao
interface TaskDao {
    @Query(
        """
        SELECT * FROM tasks
        WHERE isArchived = 0 AND isCompleted = 0
        ORDER BY isPinned DESC, dueDate IS NULL, dueDate ASC, createdAt DESC
        """
    )
    fun observeActiveTasks(): Flow<List<TaskEntity>>

    @Query(
        """
        SELECT * FROM tasks
        WHERE isCompleted = 1 AND isArchived = 0
        ORDER BY updatedAt DESC
        """
    )
    fun observeCompletedTasks(): Flow<List<TaskEntity>>

    @Query(
        """
        SELECT * FROM tasks
        WHERE isArchived = 1
        ORDER BY updatedAt DESC
        """
    )
    fun observeArchivedTasks(): Flow<List<TaskEntity>>

    @Query("SELECT * FROM tasks ORDER BY updatedAt DESC")
    fun observeAllTasks(): Flow<List<TaskEntity>>

    @Query(
        """
        SELECT * FROM tasks
        WHERE isArchived = 0
            AND (title LIKE '%' || :query || '%' OR description LIKE '%' || :query || '%')
        ORDER BY isPinned DESC, updatedAt DESC
        """
    )
    fun searchTasks(query: String): Flow<List<TaskEntity>>

    @Query("SELECT * FROM tasks WHERE id = :id LIMIT 1")
    fun getTaskById(id: Long): Flow<TaskEntity?>

    @Query("SELECT * FROM tasks WHERE id = :id LIMIT 1")
    suspend fun getTaskByIdNow(id: Long): TaskEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertTask(entity: TaskEntity): Long

    @Update
    suspend fun updateTask(entity: TaskEntity)

    @Query("UPDATE tasks SET isArchived = 0, updatedAt = :updatedAt WHERE id = :id")
    suspend fun unarchiveTask(id: Long, updatedAt: Long)

    @Delete
    suspend fun deleteTask(entity: TaskEntity)

    @Query("DELETE FROM tasks WHERE id = :id")
    suspend fun deleteTaskById(id: Long)
}

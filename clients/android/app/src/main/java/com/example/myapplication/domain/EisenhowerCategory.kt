package com.example.myapplication.domain

enum class EisenhowerCategory(
    val displayName: String,
    val isImportant: Boolean,
    val isUrgent: Boolean,
) {
    DO_NOW(
        displayName = "Do Now",
        isImportant = true,
        isUrgent = true,
    ),
    SCHEDULE(
        displayName = "Schedule",
        isImportant = true,
        isUrgent = false,
    ),
    DELEGATE_WAITING(
        displayName = "Delegate / Waiting",
        isImportant = false,
        isUrgent = true,
    ),
    ELIMINATE_LATER(
        displayName = "Eliminate / Later",
        isImportant = false,
        isUrgent = false,
    );

    companion object {
        fun from(isImportant: Boolean, isUrgent: Boolean): EisenhowerCategory = when {
            isImportant && isUrgent -> DO_NOW
            isImportant && !isUrgent -> SCHEDULE
            !isImportant && isUrgent -> DELEGATE_WAITING
            else -> ELIMINATE_LATER
        }
    }
}

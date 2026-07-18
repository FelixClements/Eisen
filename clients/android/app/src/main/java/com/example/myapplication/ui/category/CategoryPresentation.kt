package com.example.myapplication.ui.category

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ForwardToInbox
import androidx.compose.material.icons.filled.Event
import androidx.compose.material.icons.filled.LowPriority
import androidx.compose.material.icons.filled.PriorityHigh
import androidx.compose.ui.graphics.vector.ImageVector
import com.example.myapplication.domain.EisenhowerCategory

data class CategoryPresentation(
    val icon: ImageVector,
    val label: String,
    val selectorLabel: String,
    val shortcutLabel: String,
)

fun EisenhowerCategory.presentation(): CategoryPresentation = when (this) {
    EisenhowerCategory.DO_NOW -> CategoryPresentation(
        icon = Icons.Filled.PriorityHigh,
        label = "Do Now",
        selectorLabel = "Do Now",
        shortcutLabel = "Q",
    )
    EisenhowerCategory.SCHEDULE -> CategoryPresentation(
        icon = Icons.Filled.Event,
        label = "Schedule",
        selectorLabel = "Schedule",
        shortcutLabel = "W",
    )
    EisenhowerCategory.DELEGATE_WAITING -> CategoryPresentation(
        icon = Icons.AutoMirrored.Filled.ForwardToInbox,
        label = "Delegate / Waiting",
        selectorLabel = "Delegate",
        shortcutLabel = "E",
    )
    EisenhowerCategory.ELIMINATE_LATER -> CategoryPresentation(
        icon = Icons.Filled.LowPriority,
        label = "Eliminate / Later",
        selectorLabel = "Eliminate",
        shortcutLabel = "R",
    )
}

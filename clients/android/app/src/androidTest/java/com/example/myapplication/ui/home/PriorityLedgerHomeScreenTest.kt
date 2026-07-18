package com.example.myapplication.ui.home

import androidx.activity.ComponentActivity
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertHasClickAction
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.accessibility.enableAccessibilityChecks
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.example.myapplication.domain.Task
import com.example.myapplication.ui.theme.StaticTestTheme
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
@OptIn(ExperimentalTestApi::class)
class PriorityLedgerHomeScreenTest {
    @get:Rule
    val composeRule = createAndroidComposeRule<ComponentActivity>().apply {
        enableAccessibilityChecks()
    }

    @Test
    fun homeExposesMaterialActionsAndConfirmsImmediateArchive() {
        val task = task()
        var archived = false

        composeRule.setContent {
            StaticTestTheme {
                PriorityLedgerHomeScreen(
                    uiState = HomeUiState(
                        activeTasks = listOf(task),
                        groupedTasks = HomeTaskSorter.groupTasks(listOf(task)),
                        isLoading = false,
                    ),
                    onTaskCompletionChange = { _, _ -> },
                    onArchiveTask = { archived = true },
                    onOpenNewTask = {},
                )
            }
        }

        composeRule.onNodeWithText("Do Now").assertIsDisplayed()
        composeRule.onNodeWithContentDescription("Add task. Keyboard shortcut A").assertHasClickAction()
        composeRule.onNodeWithContentDescription("Keyboard shortcuts").assertHasClickAction()
        composeRule.onNodeWithContentDescription("Archive Pay electricity bill")
            .assertHasClickAction()
            .performClick()

        composeRule.runOnIdle {
            assertTrue(archived)
        }
        composeRule.onNodeWithText("Archived").assertIsDisplayed()
    }

    private fun task(): Task = Task(
        id = 1L,
        title = "Pay electricity bill",
        description = null,
        isImportant = true,
        isUrgent = true,
        dueDate = null,
        isCompleted = false,
        isArchived = false,
        isPinned = false,
        category = null,
        createdAt = 0L,
        updatedAt = 0L,
    )
}

package com.example.myapplication.ui.navigation

import androidx.activity.ComponentActivity
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertHasClickAction
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.accessibility.enableAccessibilityChecks
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithContentDescription
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.example.myapplication.R
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.domain.Task
import com.example.myapplication.domain.TaskRepository
import com.example.myapplication.domain.NoOpTaskReminderScheduler
import com.example.myapplication.ui.history.HistoryViewModel
import com.example.myapplication.ui.home.HomeViewModel
import com.example.myapplication.ui.theme.StaticTestTheme
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.flowOf
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
@OptIn(ExperimentalTestApi::class)
class PriorityLedgerAppTest {
    @get:Rule
    val composeRule = createAndroidComposeRule<ComponentActivity>().apply {
        enableAccessibilityChecks()
    }

    @Test
    fun drawerNavigationUpdatesSelectedDestinationAndClosesAfterSelection() {
        setApp()

        openDrawer()
        drawerItem(PriorityLedgerRoutes.LEDGER).assertIsSelected()

        drawerItem(PriorityLedgerRoutes.HISTORY).performClick()

        composeRule.onNodeWithText("Completed and Archive views are forthcoming.")
            .assertIsDisplayed()
        assertDrawerState("Closed")

        openDrawer()
        drawerItem(PriorityLedgerRoutes.HISTORY).assertIsSelected()

        drawerItem(PriorityLedgerRoutes.SETTINGS).performClick()

        composeRule.onNodeWithText("Settings are forthcoming.").assertIsDisplayed()
        assertDrawerState("Closed")

        openDrawer()
        drawerItem(PriorityLedgerRoutes.SETTINGS).assertIsSelected()
    }

    @Test
    fun backClosesTheDrawerWithoutLeavingLedger() {
        setApp()

        openDrawer()
        composeRule.runOnIdle {
            composeRule.activity.onBackPressedDispatcher.onBackPressed()
        }
        composeRule.waitForIdle()

        composeRule.onNodeWithText("Active tasks").assertIsDisplayed()
        assertDrawerState("Closed")
    }

    @Test
    fun backClosesTheDrawerWithoutLeavingHistory() {
        setApp()
        navigateTo(PriorityLedgerRoutes.HISTORY)

        openDrawer()
        pressBack()

        composeRule.onNodeWithText("Completed and Archive views are forthcoming.")
            .assertIsDisplayed()
        assertDrawerState("Closed")
    }

    @Test
    fun backClosesTheDrawerWithoutLeavingSettings() {
        setApp()
        navigateTo(PriorityLedgerRoutes.SETTINGS)

        openDrawer()
        pressBack()

        composeRule.onNodeWithText("Settings are forthcoming.").assertIsDisplayed()
        assertDrawerState("Closed")
    }

    @Test
    fun keyboardShortcutsAreReachableFromDrawer() {
        setApp()

        openDrawer()
        composeRule.onNodeWithText("Keyboard shortcuts").performClick()

        composeRule.onNodeWithText("New Task composer").assertIsDisplayed()
        composeRule.onNodeWithText("Alt + Q / W / E / R").assertIsDisplayed()
    }

    @Test
    fun drawerItemNavigatesWithEnterKey() {
        setApp()

        openDrawer()
        drawerItem(PriorityLedgerRoutes.HISTORY)
            .performSemanticsAction(SemanticsActions.RequestFocus) { requestFocus ->
                requestFocus()
            }
        drawerItem(PriorityLedgerRoutes.HISTORY).assertIsFocused()
        drawerItem(PriorityLedgerRoutes.HISTORY).performKeyInput {
            keyDown(Key.Enter)
            keyUp(Key.Enter)
        }

        composeRule.onNodeWithText("Completed and Archive views are forthcoming.")
            .assertIsDisplayed()
        assertDrawerState("Closed")
    }

    @Test
    fun visibleIdentityUsesPriorityLedger() {
        setApp()

        composeRule.onNodeWithText("Active tasks").assertIsDisplayed()
        assertEquals("Priority Ledger", composeRule.activity.getString(R.string.app_name))

        openDrawer()
        composeRule.onNodeWithText("Priority Ledger").assertIsDisplayed()
    }

    @Test
    fun drawerEntriesExposeAccessibleLabelsAndIcons() {
        setApp()

        openDrawer()

        listOf(
            "Ledger" to PriorityLedgerRoutes.LEDGER,
            "History" to PriorityLedgerRoutes.HISTORY,
            "Settings" to PriorityLedgerRoutes.SETTINGS,
            "Keyboard shortcuts" to PriorityLedgerRoutes.KEYBOARD_SHORTCUTS,
        ).forEach { (label, route) ->
            composeRule.onNodeWithText(label)
                .assertIsDisplayed()
                .assertHasClickAction()
            drawerItem(route).assertHasClickAction()
            composeRule.onNodeWithContentDescription("$label destination").assertIsDisplayed()
        }
    }

    @Test
    fun drawerIsUnavailableForAnUnsavedDraftAndBackKeepsTheDraft() {
        setApp()

        composeRule.onNodeWithContentDescription("Add task. Keyboard shortcut A").performClick()
        composeRule.onNodeWithTag("new-task-title").performTextInput("Keep this draft")

        composeRule.onAllNodesWithContentDescription("Open navigation drawer").assertCountEquals(0)

        pressBack()

        composeRule.onNodeWithText("Discard draft?").assertIsDisplayed()
        composeRule.onNodeWithText("Keep editing").performClick()
        composeRule.onNodeWithTag("new-task-title").assertTextEquals("Keep this draft")
    }

    private fun setApp() {
        val repository = FakeTaskRepository()
        val homeViewModel = HomeViewModel(repository)
        val historyViewModel = HistoryViewModel(repository, NoOpTaskReminderScheduler)
        composeRule.setContent {
            StaticTestTheme {
                PriorityLedgerApp(
                    repository = repository,
                    homeViewModel = homeViewModel,
                    historyViewModel = historyViewModel,
                )
            }
        }
    }

    private fun openDrawer() {
        composeRule.onNodeWithContentDescription("Open navigation drawer").performClick()
        assertDrawerState("Open")
    }

    private fun navigateTo(destination: String) {
        openDrawer()
        drawerItem(destination).performClick()
    }

    private fun drawerItem(route: String) =
        composeRule.onNodeWithTag("navigation-drawer-item-$route")

    private fun pressBack() {
        composeRule.runOnIdle {
            composeRule.activity.onBackPressedDispatcher.onBackPressed()
        }
        composeRule.waitForIdle()
    }

    private fun assertDrawerState(expected: String) {
        composeRule.onNodeWithContentDescription("Navigation drawer")
            .assert(
                SemanticsMatcher.expectValue(
                    SemanticsProperties.StateDescription,
                    expected,
                ),
            )
    }
}

private class FakeTaskRepository : TaskRepository {
    private val activeTasks = MutableStateFlow(emptyList<Task>())

    override fun observeActiveTasks(): Flow<List<Task>> = activeTasks

    override fun observeCompletedTasks(): Flow<List<Task>> = flowOf(emptyList())

    override fun observeArchivedTasks(): Flow<List<Task>> = flowOf(emptyList())

    override fun observeAllTasks(): Flow<List<Task>> = flowOf(emptyList())

    override fun searchTasks(query: String): Flow<List<Task>> = flowOf(emptyList())

    override fun getTaskById(id: Long): Flow<Task?> = flowOf(null)

    override suspend fun addTask(task: Task): Long = 1L

    override suspend fun updateTask(task: Task) = Unit

    override suspend fun completeTask(id: Long, isCompleted: Boolean) = Unit

    override suspend fun archiveTask(id: Long) = Unit

    override suspend fun unarchiveTask(id: Long) = Unit

    override suspend fun deleteTask(id: Long) = Unit

    override suspend fun moveTask(id: Long, category: EisenhowerCategory) = Unit
}

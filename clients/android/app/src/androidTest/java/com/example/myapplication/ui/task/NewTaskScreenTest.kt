package com.example.myapplication.ui.task

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.requiredWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.junit4.accessibility.enableAccessibilityChecks
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.example.myapplication.domain.EisenhowerCategory
import com.example.myapplication.ui.theme.StaticTestTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
@OptIn(ExperimentalTestApi::class)
class NewTaskScreenTest {

    @get:Rule
    val composeRule = createAndroidComposeRule<ComponentActivity>().apply {
        enableAccessibilityChecks()
    }

    @Test
    fun blankSaveShowsPersistentErrorAndFocusesTitle() {
        var saveCount = 0

        setComposer(onSave = { saveCount++ })

        composeRule.onNodeWithTag("new-task-save").performClick()

        composeRule.onNodeWithText("Task title is required.").assertIsDisplayed()
        composeRule.onNodeWithTag("new-task-title").assertIsFocused()
        assertEquals(0, saveCount)
    }

    @Test
    fun categoryCardsHaveExclusiveRadioSelection() {
        setComposer()

        composeRule.onNodeWithTag("new-task-category-DO_NOW")
            .assert(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.RadioButton))
            .assertIsSelected()
        composeRule.onNodeWithText("Schedule").assertIsDisplayed()

        composeRule.onNodeWithTag("new-task-category-SCHEDULE").performClick()

        composeRule.onNodeWithTag("new-task-category-DO_NOW").assertIsNotSelected()
        composeRule.onNodeWithTag("new-task-category-SCHEDULE").assertIsSelected()
    }

    @Test
    fun categoryChooserUsesShortVisibleLabelsAndCanonicalRadioDescriptions() {
        setComposer()

        composeRule.onNodeWithText("Do Now").assertIsDisplayed()
        composeRule.onNodeWithText("Schedule").assertIsDisplayed()
        composeRule.onNodeWithText("Delegate").assertIsDisplayed()
        composeRule.onNodeWithText("Eliminate").assertIsDisplayed()
        composeRule.onNodeWithContentDescription("Delegate / Waiting")
            .assert(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.RadioButton))
            .assertIsDisplayed()
        composeRule.onNodeWithContentDescription("Eliminate / Later")
            .assert(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.RadioButton))
            .assertIsDisplayed()

        composeRule.onNodeWithContentDescription("More actions").performClick()
        composeRule.onNodeWithText("Keyboard shortcuts").performClick()
        composeRule.onNodeWithText("Alt + Q").assertIsDisplayed()
        composeRule.onNodeWithText("Alt + W").assertIsDisplayed()
        composeRule.onNodeWithText("Alt + E").assertIsDisplayed()
        composeRule.onNodeWithText("Alt + R").assertIsDisplayed()
    }

    @Test
    fun categorySelectorUsesAvailable573DpFormWidth() {
        setComposer(
            hostModifier = Modifier
                .requiredWidth(573.dp)
                .fillMaxHeight(),
        )

        composeRule.onNodeWithTag("new-task-category-DO_NOW")
            .assert(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.RadioButton))
            .assertIsSelected()

        val doNowBounds = composeRule.onNodeWithTag("new-task-category-DO_NOW")
            .fetchSemanticsNode()
            .boundsInRoot
        val scheduleBounds = composeRule.onNodeWithTag("new-task-category-SCHEDULE")
            .fetchSemanticsNode()
            .boundsInRoot
        val delegateBounds = composeRule.onNodeWithTag("new-task-category-DELEGATE_WAITING")
            .fetchSemanticsNode()
            .boundsInRoot
        val eliminateBounds = composeRule.onNodeWithTag("new-task-category-ELIMINATE_LATER")
            .fetchSemanticsNode()
            .boundsInRoot
        val titleBounds = composeRule.onNodeWithTag("new-task-title")
            .fetchSemanticsNode()
            .boundsInRoot
        val expectedFormWidth = with(composeRule.density) { 573.dp.toPx() }
        val expectedContentWidth = with(composeRule.density) { 525.dp.toPx() }
        val expectedGutter = with(composeRule.density) { 24.dp.toPx() }
        val expectedSegmentWidth = with(composeRule.density) { 131.25.dp.toPx() }
        val expectedSegmentHeight = with(composeRule.density) { 64.dp.toPx() }

        assertEquals(doNowBounds.top, scheduleBounds.top, 0.5f)
        assertTrue(scheduleBounds.left > doNowBounds.left)
        assertEquals(doNowBounds.top, delegateBounds.top, 0.5f)
        assertEquals(doNowBounds.top, eliminateBounds.top, 0.5f)
        assertTrue(delegateBounds.left > scheduleBounds.left)
        assertTrue(eliminateBounds.left > delegateBounds.left)
        assertEquals(expectedSegmentWidth, doNowBounds.width, 0.5f)
        assertEquals(expectedSegmentWidth, scheduleBounds.width, 0.5f)
        assertEquals(expectedSegmentWidth, delegateBounds.width, 0.5f)
        assertEquals(expectedSegmentWidth, eliminateBounds.width, 0.5f)
        assertEquals(expectedContentWidth, titleBounds.width, 0.5f)
        assertEquals(expectedContentWidth, eliminateBounds.right - doNowBounds.left, 0.5f)
        assertEquals(expectedGutter, doNowBounds.left, 0.5f)
        assertEquals(expectedFormWidth - expectedGutter, eliminateBounds.right, 0.5f)
        assertEquals(
            expectedFormWidth,
            eliminateBounds.right - doNowBounds.left + (2 * expectedGutter),
            0.5f,
        )
        assertEquals(expectedSegmentHeight, doNowBounds.height, 0.5f)
    }

    @Test
    fun darkSegmentedSelectorUsesCategorySelectionSemantics() {
        setComposer(darkTheme = true)

        composeRule.onNodeWithTag("new-task-category-DO_NOW")
            .assert(SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.RadioButton))
            .assertIsSelected()
        composeRule.onNodeWithTag("new-task-category-SCHEDULE").performClick()
        composeRule.onNodeWithTag("new-task-category-DO_NOW").assertIsNotSelected()
        composeRule.onNodeWithTag("new-task-category-SCHEDULE").assertIsSelected()
    }

    @Test
    fun categoryChangeRequiresDiscardConfirmationBeforeCancelling() {
        var cancelCount = 0

        setComposer(onCancel = { cancelCount++ })

        composeRule.onNodeWithTag("new-task-category-SCHEDULE").performClick()
        composeRule.onNodeWithContentDescription("More actions").performClick()
        composeRule.onNodeWithText("Cancel").performClick()

        composeRule.onNodeWithText("Discard draft?").assertIsDisplayed()
        assertEquals(0, cancelCount)

        composeRule.onNodeWithText("Discard").performClick()
        assertEquals(1, cancelCount)
    }

    @Test
    fun enterInTitleSavesTask() {
        var savedTitle: String? = null

        setComposer(onSave = { savedTitle = it })

        composeRule.onNodeWithTag("new-task-title").performTextInput("Write report")
        composeRule.onNodeWithTag("new-task-title").performKeyInput {
            keyDown(Key.Enter)
            keyUp(Key.Enter)
        }

        composeRule.runOnIdle {
            assertEquals("Write report", savedTitle)
        }
    }

    @Test
    fun altCategoryShortcutSelectsCategoryWhileTitleIsFocusedWithoutChangingTitle() {
        setComposer()

        composeRule.onNodeWithTag("new-task-title").performTextInput("Write report")
        composeRule.onNodeWithTag("new-task-title").assertIsFocused()
        composeRule.onNodeWithTag("new-task-title").performKeyInput {
            keyDown(Key.AltLeft)
            keyDown(Key.E)
            keyUp(Key.E)
            keyUp(Key.AltLeft)
        }

        composeRule.onNodeWithTag("new-task-category-DELEGATE_WAITING").assertIsSelected()
        composeRule.onNodeWithTag("new-task-title").assertTextEquals("Write report")
    }

    @Test
    fun enterInNotesKeepsNewlineInputAndDoesNotSave() {
        var saveCount = 0

        setComposer(onSave = { saveCount++ })

        composeRule.onNodeWithTag("new-task-notes").performTextInput("First line")
        composeRule.onNodeWithTag("new-task-notes").performKeyInput {
            keyDown(Key.Enter)
            keyUp(Key.Enter)
        }

        composeRule.onNodeWithTag("new-task-notes").assertTextContains("First line\n")
        assertEquals(0, saveCount)
    }

    private fun setComposer(
        hostModifier: Modifier = Modifier.fillMaxSize(),
        darkTheme: Boolean = false,
        onSave: (String) -> Unit = {},
        onCancel: () -> Unit = {},
    ) {
        composeRule.setContent {
            StaticTestTheme(darkTheme = darkTheme) {
                Box(
                    modifier = hostModifier,
                ) {
                    Composer(
                        onSave = onSave,
                        onCancel = onCancel,
                    )
                }
            }
        }
    }

    @Composable
    private fun Composer(
        onSave: (String) -> Unit,
        onCancel: () -> Unit,
    ) {
        NewTaskScreen(
            defaultCategory = EisenhowerCategory.DO_NOW,
            onSaveTask = { title, _, _, _, _, _, onDone ->
                onSave(title)
                onDone(Result.success(0L))
            },
            onCancel = onCancel,
        )
    }
}

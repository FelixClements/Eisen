package com.example.myapplication.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

val TopBarHeight = 56.dp

@Composable
fun ScreenTopBar(
    title: @Composable () -> Unit,
    modifier: Modifier = Modifier,
    navigationIcon: @Composable (() -> Unit) = {},
    actions: @Composable RowScope.() -> Unit = {},
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .statusBarsPadding(),
        contentAlignment = Alignment.BottomCenter,
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .height(TopBarHeight)
                .padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                modifier = Modifier.weight(1f),
                contentAlignment = Alignment.CenterStart,
            ) {
                navigationIcon()
            }
            Box(
                modifier = Modifier.weight(1f),
                contentAlignment = Alignment.Center,
            ) {
                title()
            }
            Box(
                modifier = Modifier.weight(1f),
                contentAlignment = Alignment.CenterEnd,
            ) {
                Row(
                    horizontalArrangement = Arrangement.End,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    actions()
                }
            }
        }
    }
}

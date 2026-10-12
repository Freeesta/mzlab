# Accessibility

mzLab aims at **WCAG 2.2 level AA** for Data, Drawing, Theory and Practice. Not everything is guaranteed: the chemical editor (Ketcher) is a third-party component with its own limits.

## What is provided
- **Keyboard**: the controls can be reached by keyboard; buttons have names for screen readers (`aria-label`). In charts, zoom and pan also work without dragging (buttons and shortcuts; Backspace returns to the full view).
- **Colours**: only theme colours, light and dark theme, text contrast checked in both themes; charts use a palette readable by people with colour-vision deficiency (Okabe-Ito).
- **Every chart has its data table**, for those who cannot read the drawing.
- **Targets**: controls are at least 24×24 px (40 px on touch screens).
- **Motion**: if the system asks to reduce motion, animations are reduced.
- **Text**: size is set in the settings (gear icon); the interface is in Italian and English (Theory and Practice in Italian only).

## How it was checked
Automated checks (axe) on the charts and the main screens. There is not yet a review with people who use screen readers.

## If you find a barrier
Report it with an [issue](https://github.com/Freeesta/mzlab/issues/new/choose): describe the browser, the device and the assistive tool you use. Best-effort support, no guaranteed response time. If you cannot use part of the program during a lesson, ask your teacher for an alternative.

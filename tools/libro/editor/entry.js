// Entry of the vendored editor bundle: only the Crepe features the Libro needs (no images, no AI, no top bar).
import { CrepeBuilder } from "@milkdown/crepe/builder";
import { blockEdit } from "@milkdown/crepe/feature/block-edit";
import { toolbar } from "@milkdown/crepe/feature/toolbar";
import { latex } from "@milkdown/crepe/feature/latex";
import { codeMirror } from "@milkdown/crepe/feature/code-mirror";
import { table } from "@milkdown/crepe/feature/table";
import { listItem } from "@milkdown/crepe/feature/list-item";
import { linkTooltip } from "@milkdown/crepe/feature/link-tooltip";
import { cursor } from "@milkdown/crepe/feature/cursor";
import { placeholder } from "@milkdown/crepe/feature/placeholder";

window.LibroEditor = { CrepeBuilder, features: { blockEdit, toolbar, latex, codeMirror, table, listItem, linkTooltip, cursor, placeholder } };

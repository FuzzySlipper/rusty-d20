/**
 * Shell route paths owned by the Rusty D20 product.
 *
 * The shell layer owns the route *map*. The application (`apps/app`, tagged
 * `type:app`) binds these paths to the feature screen components — libraries
 * (`type:lib`) may not depend on feature libraries (`type:feature`), so the
 * route binding itself lives in the app while the paths stay here.
 *
 * Later gameplay milestones add routes here rather than establishing another shell.
 */
export const SHELL_PATHS = {
  mainMenu: '',
} as const;

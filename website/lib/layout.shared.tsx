import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { VscVscode } from 'react-icons/vsc';

export const gitConfig = {
  user: 'buildshit',
  repo: 'opendaemon',
  branch: 'main',
};

/** VS Code Marketplace icon */
const VSCodeIcon = <VscVscode size={16} />;

/** Open VSX icon */
const OpenVSXIcon = (
  <svg width="16" height="16" viewBox="0 0 337.52 432.07" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
    <path d="M633.14,438.09c-.73,2.28-2.67,1.6-4.21,1.6q-75.46,0-150.94.1c-5.15,0-3.53-2-2.05-4.58Q505,384.83,534.08,334.43q8.35-14.49,16.77-28.95c.79-1.35,1-3.3,3.18-3.45,20.39,35.41,40.66,70.9,61.24,106.2C621.11,418.24,626.14,428.77,633.14,438.09Z" transform="translate(-474.47 -145.05)"/>
    <path d="M564.08,282.93c-2.26.09-2.46-1.95-3.23-3.28q-37.71-65-75.34-130c-.72-1.24-1.89-2.38-1.5-4.41h4.84q73.71,0,147.43,0c1.9,0,4-.76,5.73.77-1.48,4.44-4.23,8.21-6.53,12.2-19.26,33.32-38.63,66.57-57.76,100C573.05,266.34,567.54,274.08,564.08,282.93Z" transform="translate(-474.47 -145.05)"/>
    <path d="M811.13,437.78c2.23,2.82-.44,4.72-1.53,6.62Q783.37,490,756.91,535.56q-10.24,17.7-20.49,35.42c-1.06,1.84-2.2,3.65-3.71,6.14-6.64-11.46-13-22.31-19.23-33.17q-29-50.1-57.94-100.24c-.81-1.39-2.73-2.65-1.52-4.76q75.51,0,151-.09C807.07,438.86,809.39,439.74,811.13,437.78Z" transform="translate(-474.47 -145.05)"/>
    <path d="M564.08,282.93c3.46-8.85,9-16.59,13.64-24.76,19.13-33.39,38.5-66.64,57.76-100,2.3-4,5-7.76,6.53-12.2,4.62,5.68,7.71,12.29,11.34,18.56q33.15,57.15,66.16,114.37c.64,1.12,1.23,2.26,1.84,3.38-.53.35-.78.66-1,.66Q642.2,283,564.08,282.93Z" transform="translate(-474.47 -145.05)"/>
    <path d="M811.13,437.78c-1.74,2-4.06,1.08-6.1,1.08q-75.51.11-151,.09c8.36-14.74,16.64-29.52,25.09-44.21q26.5-46.08,53.1-92c1.19-.06,1.5.72,1.91,1.42Z" transform="translate(-474.47 -145.05)"/>
    <path d="M633.14,438.09c-7-9.32-12-19.85-17.87-29.86C594.69,372.93,574.42,337.44,554,302h0c2,0,4,.12,6,.12q72.5,0,145,0c2,0,4.34-.85,6.14,1Z" transform="translate(-474.47 -145.05)"/>
    <path d="M711.15,303.17c-1.8-1.84-4.09-1-6.14-1q-72.5-.07-145,0c-2,0-4-.08-6-.12,1.59-1.39,3.5-.75,5.26-.75q73.76-.06,147.51.1C708.1,301.37,711.24,299.08,711.15,303.17Z" transform="translate(-474.47 -145.05)"/>
  </svg>
);

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: 'OpenDaemon',
    },
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
    links: [
      {
        icon: VSCodeIcon,
        text: 'VS Code',
        url: 'https://marketplace.visualstudio.com/items?itemName=opendaemon.opendaemon',
        type: 'icon',
      },
      {
        icon: OpenVSXIcon,
        text: 'Open VSX',
        url: 'https://open-vsx.org/extension/opendaemon/opendaemon',
        type: 'icon',
      },
    ],
  };
}
